use helm_forge::{
    ChartGenerator, DefaultAttributeFilter, GenerationStage, HelmBackend, HelmConfig,
    ValuesGenerator,
    generate_chart_yaml, generate_configmap_template, generate_deployment_test,
    generate_secret_template, generate_values_schema, generate_values_yaml,
    iac_type_to_json_schema,
};
use iac_forge::backend::Backend;
use iac_forge::testing::{test_provider, test_resource, test_resource_with_type, TestAttributeBuilder};
use iac_forge::IacType;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

// ── End-to-end generation ───────────────────────────────────────────────────

#[test]
fn generate_complete_chart_and_write_to_disk() {
    let backend = HelmBackend::default();
    let provider = test_provider("akeyless");
    let resource = test_resource("static_secret");

    let artifacts = backend
        .generate_resource(&resource, &provider)
        .expect("generation failed");
    assert!(artifacts.len() >= 11);

    let tmpdir = TempDir::new().unwrap();
    for artifact in &artifacts {
        let path = tmpdir.path().join(&artifact.path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, &artifact.content).unwrap();
    }

    // Verify directory structure
    let base = tmpdir.path().join("charts/static-secret");
    assert!(base.join("Chart.yaml").exists());
    assert!(base.join("values.yaml").exists());
    assert!(base.join("values.schema.json").exists());
    assert!(base.join("templates/_helpers.tpl").exists());
    assert!(base.join("templates/deployment.yaml").exists());
    assert!(base.join("templates/service.yaml").exists());
    assert!(base.join("templates/serviceaccount.yaml").exists());
    assert!(base.join("templates/servicemonitor.yaml").exists());
    assert!(base.join("templates/networkpolicy.yaml").exists());
    assert!(base.join("templates/pdb.yaml").exists());
    assert!(base.join("templates/hpa.yaml").exists());
    assert!(base.join("templates/podmonitor.yaml").exists());
    assert!(base.join("templates/configmap.yaml").exists());
    assert!(base.join("templates/secret.yaml").exists());

    // Count template files
    let count = fs::read_dir(base.join("templates")).unwrap().count();
    assert!(count >= 10, "expected >=10 templates, got {count}");
}

#[test]
fn all_templates_have_valid_helm_syntax() {
    let backend = HelmBackend::default();
    let provider = test_provider("test");
    let resource = test_resource("test_res");
    let artifacts = backend.generate_resource(&resource, &provider).unwrap();

    for artifact in artifacts.iter().filter(|a| a.path.contains("/templates/")) {
        let open = artifact.content.matches("{{").count();
        let close = artifact.content.matches("}}").count();
        assert_eq!(
            open, close,
            "unbalanced braces in {}: {open} open, {close} close",
            artifact.path
        );
        assert!(
            !artifact.content.contains("{{{"),
            "triple open braces in {}",
            artifact.path
        );
        assert!(
            !artifact.content.contains("}}}"),
            "triple close braces in {}",
            artifact.path
        );
    }
}

#[test]
fn chart_yaml_is_parseable_yaml() {
    let resource = test_resource("static_secret");
    let yaml_str = generate_chart_yaml(&resource, "akeyless");
    let doc: serde_json::Value = serde_yaml::from_str(&yaml_str).expect("Chart.yaml must be valid YAML");
    assert_eq!(doc["apiVersion"], "v2");
    assert_eq!(doc["name"], "static-secret");
    assert_eq!(doc["type"], "application");
}

#[test]
fn values_schema_is_valid_json() {
    let resource = test_resource("static_secret");
    let json_str = generate_values_schema(&resource);
    let schema: Value = serde_json::from_str(&json_str).expect("schema must be valid JSON");
    assert_eq!(schema["type"], "object");
    assert!(schema["$schema"].is_string());
}

// ── Determinism ─────────────────────────────────────────────────────────────

#[test]
fn generation_is_deterministic() {
    let backend = HelmBackend::default();
    let provider = test_provider("akeyless");
    let resource = test_resource("determinism_test");

    let run1 = backend.generate_resource(&resource, &provider).unwrap();
    let run2 = backend.generate_resource(&resource, &provider).unwrap();

    assert_eq!(run1.len(), run2.len());
    for (a, b) in run1.iter().zip(run2.iter()) {
        assert_eq!(a.path, b.path, "artifact paths differ");
        assert_eq!(a.content, b.content, "artifact content differs for {}", a.path);
    }
}

// ── Custom config propagation ───────────────────────────────────────────────

#[test]
fn all_config_fields_propagate_to_output() {
    let config = HelmConfig {
        lib_chart_name: "custom-lib".into(),
        lib_chart_version: "~2.0.0".into(),
        lib_chart_repository: "https://charts.example.com".into(),
        default_chart_version: "3.4.5".into(),
        default_app_version: "9.8.7".into(),
        cpu_request: "999m".into(),
        memory_request: "999Mi".into(),
        cpu_limit: "9999m".into(),
        memory_limit: "9999Mi".into(),
    };
    let backend = HelmBackend::with_config(config);
    let provider = test_provider("test");
    let resource = test_resource("test_res");

    let artifacts = backend.generate_resource(&resource, &provider).unwrap();

    let chart = &artifacts.iter().find(|a| a.path.ends_with("Chart.yaml")).unwrap().content;
    assert!(chart.contains("custom-lib"), "lib_chart_name");
    assert!(chart.contains("~2.0.0"), "lib_chart_version");
    assert!(chart.contains("https://charts.example.com"), "lib_chart_repository");
    assert!(chart.contains("version: 3.4.5"), "default_chart_version");
    assert!(chart.contains("\"9.8.7\""), "default_app_version");

    let values = &artifacts.iter().find(|a| a.path.ends_with("values.yaml")).unwrap().content;
    assert!(values.contains("999m"), "cpu_request");
    assert!(values.contains("999Mi"), "memory_request");
    assert!(values.contains("9999m"), "cpu_limit");
    assert!(values.contains("9999Mi"), "memory_limit");
}

#[test]
fn default_vs_custom_config_differ() {
    let provider = test_provider("test");
    let resource = test_resource("test_res");

    let default_artifacts = HelmBackend::default()
        .generate_resource(&resource, &provider).unwrap();
    let custom_artifacts = HelmBackend::with_config(HelmConfig {
        cpu_request: "777m".into(),
        ..HelmConfig::default()
    })
        .generate_resource(&resource, &provider).unwrap();

    assert_eq!(default_artifacts.len(), custom_artifacts.len());

    let dv = &default_artifacts.iter().find(|a| a.path.ends_with("values.yaml")).unwrap().content;
    let cv = &custom_artifacts.iter().find(|a| a.path.ends_with("values.yaml")).unwrap().content;
    assert!(dv.contains("50m"));
    assert!(cv.contains("777m"));
    assert_ne!(dv, cv);
}

// ── Edge cases: empty / computed / sensitive-only resources ──────────────────

#[test]
fn resource_with_no_attributes() {
    let mut resource = test_resource("empty");
    resource.attributes.clear();

    let backend = HelmBackend::default();
    let provider = test_provider("test");
    let artifacts = backend.generate_resource(&resource, &provider).unwrap();

    // Should still produce core artifacts (Chart, values, schema, helpers, delegates)
    assert!(artifacts.iter().any(|a| a.path.ends_with("Chart.yaml")));
    assert!(artifacts.iter().any(|a| a.path.ends_with("values.yaml")));
    assert!(artifacts.iter().any(|a| a.path.ends_with("values.schema.json")));

    // Should NOT produce configmap or secret templates
    assert!(!artifacts.iter().any(|a| a.path.ends_with("configmap.yaml")));
    assert!(!artifacts.iter().any(|a| a.path.ends_with("secret.yaml")));
}

#[test]
fn resource_with_only_computed_attributes() {
    let mut resource = test_resource_with_type("computed", "output_id", IacType::String);
    resource.attributes[0].computed = true;

    let values = generate_values_yaml(&resource);
    assert!(!values.contains("config:"), "computed attrs should not appear in config");
    assert!(!values.contains("secrets:"), "computed attrs should not appear in secrets");

    let configmap = generate_configmap_template(&resource);
    assert!(configmap.is_empty(), "no configmap for computed-only resource");

    let secret = generate_secret_template(&resource);
    assert!(secret.is_empty(), "no secret for computed-only resource");
}

#[test]
fn resource_with_only_sensitive_attributes() {
    let mut resource = test_resource_with_type("all_secret", "api_key", IacType::String);
    resource.attributes[0].sensitive = true;

    let values = generate_values_yaml(&resource);
    assert!(!values.contains("config:"), "sensitive-only should have no config section");
    assert!(values.contains("secrets:"), "sensitive-only should have secrets section");

    let configmap = generate_configmap_template(&resource);
    assert!(configmap.is_empty());

    let secret = generate_secret_template(&resource);
    assert!(!secret.is_empty());
    assert!(secret.contains("api-key")); // kebab-cased key
}

#[test]
fn resource_with_only_non_sensitive_attributes() {
    let resource = test_resource_with_type("all_config", "setting", IacType::String);

    let values = generate_values_yaml(&resource);
    assert!(values.contains("config:"));
    assert!(!values.contains("secrets:"));

    let configmap = generate_configmap_template(&resource);
    assert!(!configmap.is_empty());

    let secret = generate_secret_template(&resource);
    assert!(secret.is_empty());
}

// ── Edge cases: all IacType variants ────────────────────────────────────────

#[test]
fn all_iac_types_produce_valid_yaml_defaults() {
    let types_and_expected: Vec<(IacType, &str)> = vec![
        (IacType::String, "\"\""),
        (IacType::Integer, "0"),
        (IacType::Float, "0.0"),
        (IacType::Boolean, "false"),
        (IacType::List(Box::new(IacType::String)), "[]"),
        (IacType::Set(Box::new(IacType::Integer)), "[]"),
        (IacType::Map(Box::new(IacType::String)), "{}"),
        (IacType::Any, "\"\""),
    ];

    for (iac_type, expected) in types_and_expected {
        let resource = test_resource_with_type("typed", "field", iac_type);
        let values = generate_values_yaml(&resource);
        assert!(
            values.contains(expected),
            "IacType default for field should contain {expected}"
        );
    }
}

#[test]
fn all_iac_types_produce_valid_json_schema() {
    let types: Vec<(IacType, &str)> = vec![
        (IacType::String, "string"),
        (IacType::Integer, "integer"),
        (IacType::Float, "number"),
        (IacType::Boolean, "boolean"),
        (IacType::Any, ""),
    ];

    for (iac_type, expected_type) in types {
        let schema = iac_type_to_json_schema(&iac_type);
        if !expected_type.is_empty() {
            assert_eq!(schema["type"], expected_type, "IacType schema mismatch");
        }
    }
}

#[test]
fn deeply_nested_type_produces_valid_schema() {
    let deep = IacType::List(Box::new(IacType::Map(Box::new(IacType::List(Box::new(
        IacType::String,
    ))))));
    let schema = iac_type_to_json_schema(&deep);
    assert_eq!(schema["type"], "array");
    assert_eq!(schema["items"]["type"], "object");
    assert_eq!(schema["items"]["additionalProperties"]["type"], "array");
    assert_eq!(
        schema["items"]["additionalProperties"]["items"]["type"],
        "string"
    );
}

#[test]
fn empty_enum_produces_valid_schema() {
    let schema = iac_type_to_json_schema(&IacType::Enum {
        values: vec![],
        underlying: Box::new(IacType::String),
    });
    assert_eq!(schema["type"], "string");
    assert_eq!(schema["enum"].as_array().unwrap().len(), 0);
}

#[test]
fn enum_default_value_in_values_yaml() {
    let mut resource = test_resource("enum_test");
    resource.attributes.clear();
    resource.attributes.push(
        TestAttributeBuilder::new(
            "mode",
            IacType::Enum {
                values: vec!["fast".into(), "slow".into()],
                underlying: Box::new(IacType::String),
            },
        )
        .build(),
    );
    let values = generate_values_yaml(&resource);
    assert!(values.contains("\"fast\""), "enum default should be first value");
}

// ── Edge cases: naming ──────────────────────────────────────────────────────

#[test]
fn very_long_resource_name() {
    let long_name = "this_is_a_very_long_resource_name_that_tests_kebab_case_conversion";
    let resource = test_resource(long_name);
    let chart = generate_chart_yaml(&resource, "test");
    assert!(chart.contains("this-is-a-very-long-resource-name-that-tests-kebab-case-conversion"));
}

#[test]
fn configmap_keys_use_hyphens_not_underscores() {
    let resource = test_resource("key_test");
    let tpl = generate_configmap_template(&resource);
    if !tpl.is_empty() {
        for line in tpl.lines() {
            if line.starts_with("  ") && line.contains("| quote") {
                let key = line.trim().split(':').next().unwrap();
                assert!(
                    !key.contains('_'),
                    "configmap key should use hyphens: {key}"
                );
            }
        }
    }
}

// ── Schema completeness ─────────────────────────────────────────────────────

#[test]
fn schema_includes_all_pleme_lib_sections() {
    let resource = test_resource("schema_test");
    let schema_str = generate_values_schema(&resource);
    let schema: Value = serde_json::from_str(&schema_str).unwrap();
    let props = schema["properties"].as_object().unwrap();

    let required_sections = [
        "image",
        "replicaCount",
        "resources",
        "monitoring",
        "networkPolicy",
        "pdb",
        "autoscaling",
    ];
    for section in &required_sections {
        assert!(props.contains_key(*section), "schema missing {section}");
    }
}

#[test]
fn schema_required_attributes_marked_correctly() {
    let mut resource = test_resource("req_test");
    resource.attributes.clear();
    resource.attributes.push(
        TestAttributeBuilder::new("required_field", IacType::String)
            .required()
            .build(),
    );
    resource.attributes.push(
        TestAttributeBuilder::new("optional_field", IacType::String)
            .build(),
    );

    let schema_str = generate_values_schema(&resource);
    let schema: Value = serde_json::from_str(&schema_str).unwrap();
    let required = schema["properties"]["config"]["required"]
        .as_array()
        .expect("should have required array");
    assert!(required.iter().any(|v| v == "required_field"));
    assert!(!required.iter().any(|v| v == "optional_field"));
}

// ── AttributeFilter trait ───────────────────────────────────────────────────

#[test]
fn attribute_filter_trait_is_object_safe() {
    // Verify the trait can be used as a trait object
    let filter: Box<dyn helm_forge::AttributeFilter> = Box::new(DefaultAttributeFilter);
    let resource = test_resource("test");
    let _ = filter.config_attributes(&resource);
    let _ = filter.secret_attributes(&resource);
}

// ── Builder / DI / mock injection ────────────────────────────────────────────

#[derive(Debug)]
struct CountingChartGen {
    call_count: std::sync::atomic::AtomicUsize,
}

impl ChartGenerator for CountingChartGen {
    fn generate(&self, _resource: &iac_forge::ir::IacResource, _provider: &str) -> String {
        self.call_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        "apiVersion: v2\nname: counted\ntype: application\nversion: 0.0.1\n".into()
    }
}

#[test]
fn builder_mock_injection_replaces_chart_gen() {
    let counter = std::sync::Arc::new(CountingChartGen {
        call_count: std::sync::atomic::AtomicUsize::new(0),
    });

    // We can't share Arc through Box<dyn> directly, so clone into a wrapper
    #[derive(Debug)]
    struct Wrapper(std::sync::Arc<CountingChartGen>);
    impl ChartGenerator for Wrapper {
        fn generate(&self, r: &iac_forge::ir::IacResource, p: &str) -> String {
            self.0.generate(r, p)
        }
    }

    let backend = HelmBackend::builder()
        .chart_generator(Box::new(Wrapper(counter.clone())))
        .build();

    let provider = test_provider("test");
    let resource = test_resource("test");
    let artifacts = backend.generate_resource(&resource, &provider).unwrap();

    assert_eq!(
        counter.call_count.load(std::sync::atomic::Ordering::SeqCst),
        1
    );
    let chart = artifacts
        .iter()
        .find(|a| a.path.ends_with("Chart.yaml"))
        .unwrap();
    assert!(chart.content.contains("name: counted"));
}

#[test]
fn builder_default_equals_default_constructor() {
    let provider = test_provider("test");
    let resource = test_resource("test");

    let a = HelmBackend::default()
        .generate_resource(&resource, &provider)
        .unwrap();
    let b = HelmBackend::builder()
        .build()
        .generate_resource(&resource, &provider)
        .unwrap();

    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(b.iter()) {
        assert_eq!(x.path, y.path);
        assert_eq!(x.content, y.content);
    }
}

// ── GenerationStage FSM ─────────────────────────────────────────────────────

#[test]
fn fsm_walks_all_stages_in_order() {
    let mut stage = GenerationStage::Init;
    let expected = [
        GenerationStage::ChartMetadata,
        GenerationStage::Values,
        GenerationStage::Schema,
        GenerationStage::Templates,
        GenerationStage::Tests,
        GenerationStage::Done,
    ];
    for exp in &expected {
        stage = stage.next().unwrap();
        assert_eq!(stage, *exp);
    }
    assert!(stage.is_done());
    assert!(stage.next().is_none());
}

#[test]
fn fsm_all_stages_count() {
    assert_eq!(GenerationStage::ALL.len(), 7);
}

// ── Test generation ─────────────────────────────────────────────────────────

#[test]
fn test_file_references_correct_chart() {
    let resource = test_resource("my_resource");
    let test_yaml = generate_deployment_test(&resource);
    assert!(test_yaml.contains("suite: my-resource"));
    assert!(test_yaml.contains("templates/deployment.yaml"));
    assert!(test_yaml.contains("isKind"));
    assert!(test_yaml.contains("runAsNonRoot"));
}
