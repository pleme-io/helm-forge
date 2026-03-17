use helm_forge::{
    ChartGenerator, DefaultAttributeFilter, DefaultFluxCdGenerator, FluxCdConfig,
    FluxCdGenerator, GenerationStage, HelmBackend, HelmConfig, generate_chart_yaml,
    generate_configmap_template, generate_deployment_test, generate_helmrelease,
    generate_kustomization, generate_prometheusrule_template, generate_secret_template,
    generate_values_schema, generate_values_yaml, iac_type_to_json_schema, validate_dns1123,
    Dns1123Result,
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
    assert!(artifacts.len() >= 12);

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
    assert!(base.join("templates/prometheusrule.yaml").exists());
    assert!(base.join("templates/configmap.yaml").exists());
    assert!(base.join("templates/secret.yaml").exists());

    // Count template files
    let count = fs::read_dir(base.join("templates")).unwrap().count();
    assert!(count >= 11, "expected >=11 templates, got {count}");
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
    let doc: serde_json::Value = serde_yaml_ng::from_str(&yaml_str).expect("Chart.yaml must be valid YAML");
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
        replica_count: 5,
        default_image_repository: "ghcr.io/pleme-io/custom".into(),
        image_pull_policy: "IfNotPresent".into(),
        default_container_port: 9090,
        default_service_type: "NodePort".into(),
        cpu_request: "999m".into(),
        memory_request: "999Mi".into(),
        cpu_limit: "9999m".into(),
        memory_limit: "9999Mi".into(),
        monitoring_enabled: false,
        network_policy_enabled: false,
        pdb_enabled: true,
        autoscaling_enabled: true,
    };
    let backend = HelmBackend::with_config(config);
    let provider = test_provider("test");
    let resource = test_resource("test_res");

    let artifacts = backend.generate_resource(&resource, &provider).unwrap();

    // Verify via deserialization (format-agnostic, struct-level assertions)
    let chart_str = &artifacts.iter().find(|a| a.path.ends_with("Chart.yaml")).unwrap().content;
    let chart: helm_forge::ChartYaml = serde_yaml_ng::from_str(chart_str).unwrap();
    assert_eq!(chart.dependencies[0].name, "custom-lib");
    assert_eq!(chart.dependencies[0].version, "~2.0.0");
    assert_eq!(chart.dependencies[0].repository, "https://charts.example.com");
    assert_eq!(chart.version, "3.4.5");
    assert_eq!(chart.app_version, "9.8.7");

    let values_str = &artifacts.iter().find(|a| a.path.ends_with("values.yaml")).unwrap().content;
    let values: helm_forge::ValuesYaml = serde_yaml_ng::from_str(values_str).unwrap();
    assert_eq!(values.resources.requests.cpu, "999m");
    assert_eq!(values.resources.requests.memory, "999Mi");
    assert_eq!(values.resources.limits.cpu, "9999m");
    assert_eq!(values.resources.limits.memory, "9999Mi");
    assert_eq!(values.replica_count, 5);
    assert_eq!(values.image.repository, "ghcr.io/pleme-io/custom");
    assert_eq!(values.image.pull_policy, "IfNotPresent");
    assert_eq!(values.ports.len(), 1);
    assert_eq!(values.ports[0].container_port, 9090);
    assert_eq!(values.service.service_type, "NodePort");
    assert!(!values.monitoring.enabled);
    assert!(!values.monitoring.alerting.enabled);
    assert_eq!(values.monitoring.interval, "30s");
    assert_eq!(values.monitoring.port, "metrics");
    assert_eq!(values.monitoring.path, "/metrics");
    assert!(!values.network_policy.enabled);
    assert!(values.pdb.enabled);
    assert!(values.autoscaling.enabled);
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
    let types: Vec<IacType> = vec![
        IacType::String,
        IacType::Integer,
        IacType::Float,
        IacType::Boolean,
        IacType::List(Box::new(IacType::String)),
        IacType::Set(Box::new(IacType::Integer)),
        IacType::Map(Box::new(IacType::String)),
        IacType::Any,
    ];

    for iac_type in types {
        let resource = test_resource_with_type("typed", "field", iac_type);
        let yaml = generate_values_yaml(&resource);
        // Must produce valid YAML that serde can parse back
        let parsed: helm_forge::ValuesYaml =
            serde_yaml_ng::from_str(&yaml).expect("must produce valid YAML");
        assert!(parsed.config.is_some(), "config section should exist for non-sensitive attr");
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
    let yaml = generate_values_yaml(&resource);
    let parsed: helm_forge::ValuesYaml = serde_yaml_ng::from_str(&yaml).unwrap();
    let config = parsed.config.expect("should have config section");
    let mode_val = config.get("mode").expect("should have mode field");
    assert_eq!(mode_val.as_str().unwrap(), "fast", "enum default should be first value");
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
        "ports",
        "service",
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

// ── DNS-1123 validation integration tests ────────────────────────────────

#[test]
fn dns1123_valid_resource_name_generates_chart() {
    let backend = HelmBackend::default();
    let provider = test_provider("test");
    let resource = test_resource("my-valid-name");
    let artifacts = backend.generate_resource(&resource, &provider).unwrap();
    assert!(artifacts.iter().any(|a| a.path.contains("my-valid-name")));
}

#[test]
fn dns1123_invalid_name_returns_error() {
    let backend = HelmBackend::default();
    let provider = test_provider("test");
    let mut resource = test_resource("test");
    resource.name = "---".into();
    let result = backend.generate_resource(&resource, &provider);
    assert!(result.is_err());
}

#[test]
fn dns1123_long_name_truncated_in_generation() {
    let long_name = "a".repeat(70);
    let result = validate_dns1123(&long_name).unwrap();
    assert!(matches!(result, Dns1123Result::Truncated(_)));
    assert_eq!(result.name().len(), 63);
}

// ── PrometheusRule integration tests ─────────────────────────────────────

#[test]
fn prometheusrule_template_in_generated_artifacts() {
    let backend = HelmBackend::default();
    let provider = test_provider("test");
    let resource = test_resource("alert_test");
    let artifacts = backend.generate_resource(&resource, &provider).unwrap();

    let prom = artifacts
        .iter()
        .find(|a| a.path.ends_with("prometheusrule.yaml"))
        .expect("prometheusrule.yaml must be generated");
    assert!(prom.content.contains("kind: PrometheusRule"));
    assert!(prom.content.contains("monitoring.alerting.enabled"));
}

#[test]
fn prometheusrule_template_valid_yaml_structure() {
    let resource = test_resource("my_svc");
    let tpl = generate_prometheusrule_template(&resource);
    assert!(tpl.contains("monitoring.coreos.com/v1"));
    assert!(tpl.contains("severity: critical"));
    assert!(tpl.contains("for: 2m"));
    // Conditional on alerting.enabled.
    assert!(tpl.starts_with("{{- if .Values.monitoring.alerting.enabled }}"));
    assert!(tpl.trim_end().ends_with("{{- end }}"));
}

// ── HelmConfig validation integration tests ──────────────────────────────

#[test]
fn default_helm_config_passes_validation() {
    let cfg = HelmConfig::default();
    assert!(cfg.validate().is_empty());
}

#[test]
fn helm_config_validation_catches_all_errors() {
    let cfg = HelmConfig {
        lib_chart_version: String::new(),
        default_chart_version: String::new(),
        replica_count: 0,
        cpu_request: "bad-value".into(),
        memory_request: "xyz".into(),
        ..HelmConfig::default()
    };
    let errors = cfg.validate();
    assert_eq!(errors.len(), 5);
    assert!(errors.iter().any(|e| e.contains("lib_chart_version")));
    assert!(errors.iter().any(|e| e.contains("default_chart_version")));
    assert!(errors.iter().any(|e| e.contains("replica_count")));
    assert!(errors.iter().any(|e| e.contains("cpu_request")));
    assert!(errors.iter().any(|e| e.contains("memory_request")));
}

// ── Extended monitoring values integration tests ─────────────────────────

#[test]
fn monitoring_values_contain_new_fields() {
    let resource = test_resource("mon_test");
    let yaml = generate_values_yaml(&resource);
    assert!(yaml.contains("alerting:"));
    assert!(yaml.contains("interval:"));
    assert!(yaml.contains("port:"));
    assert!(yaml.contains("path:"));
}

#[test]
fn monitoring_schema_has_alerting_section() {
    let resource = test_resource("schema_mon");
    let schema_str = generate_values_schema(&resource);
    let schema: Value = serde_json::from_str(&schema_str).unwrap();

    let monitoring = &schema["properties"]["monitoring"]["properties"];
    assert!(monitoring["alerting"].is_object());
    assert!(monitoring["interval"].is_object());
    assert!(monitoring["port"].is_object());
    assert!(monitoring["path"].is_object());
}

// ── FluxCD HelmRelease generation ────────────────────────────────────────────

#[test]
fn fluxcd_helmrelease_is_valid_yaml_with_all_fields() {
    let resource = test_resource("static_secret");
    let yaml_str = generate_helmrelease(&resource, "akeyless", &FluxCdConfig::default());
    let doc: Value = serde_yaml_ng::from_str(&yaml_str).expect("HelmRelease must be valid YAML");

    assert_eq!(doc["apiVersion"], "helm.toolkit.fluxcd.io/v2");
    assert_eq!(doc["kind"], "HelmRelease");
    assert_eq!(doc["metadata"]["name"], "static-secret");
    assert_eq!(doc["metadata"]["namespace"], "akeyless-system");
    assert_eq!(doc["spec"]["interval"], "5m");
    assert_eq!(doc["spec"]["chart"]["spec"]["chart"], "charts/static-secret");
    assert_eq!(
        doc["spec"]["chart"]["spec"]["sourceRef"]["kind"],
        "GitRepository"
    );
    assert_eq!(
        doc["spec"]["chart"]["spec"]["sourceRef"]["name"],
        "helm-akeyless-gen"
    );
    assert_eq!(
        doc["spec"]["chart"]["spec"]["sourceRef"]["namespace"],
        "flux-system"
    );
    assert_eq!(doc["spec"]["chart"]["spec"]["interval"], "1h");
    assert_eq!(doc["spec"]["install"]["remediation"]["retries"], 3);
    assert_eq!(doc["spec"]["upgrade"]["remediation"]["retries"], 3);
}

#[test]
fn fluxcd_kustomization_lists_all_resources_sorted() {
    let resources = vec![
        test_resource("zzz_target"),
        test_resource("aaa_auth_method"),
        test_resource("mmm_secret"),
    ];
    let yaml_str = generate_kustomization(&resources);
    let doc: Value =
        serde_yaml_ng::from_str(&yaml_str).expect("kustomization must be valid YAML");

    assert_eq!(doc["apiVersion"], "kustomize.config.k8s.io/v1beta1");
    assert_eq!(doc["kind"], "Kustomization");
    let res_arr = doc["resources"].as_array().expect("resources should be array");
    assert_eq!(res_arr.len(), 3);
    // Verify sorted order
    assert_eq!(res_arr[0], "helmrelease-aaa-auth-method.yaml");
    assert_eq!(res_arr[1], "helmrelease-mmm-secret.yaml");
    assert_eq!(res_arr[2], "helmrelease-zzz-target.yaml");
}

#[test]
fn backend_with_fluxcd_produces_helmrelease_artifacts() {
    let backend = HelmBackend::builder()
        .fluxcd_config(FluxCdConfig::default())
        .build();
    let provider = test_provider("akeyless");
    let resource = test_resource("static_secret");

    let artifacts = backend
        .generate_resource(&resource, &provider)
        .unwrap();

    let fluxcd_artifact = artifacts
        .iter()
        .find(|a| a.path.contains("fluxcd/"))
        .expect("should produce a fluxcd artifact");
    assert_eq!(
        fluxcd_artifact.path,
        "fluxcd/helmrelease-static-secret.yaml"
    );
    assert!(fluxcd_artifact.content.contains("kind: HelmRelease"));
    assert!(fluxcd_artifact.content.contains("name: static-secret"));
}

#[test]
fn backend_without_fluxcd_produces_no_fluxcd_artifacts() {
    let backend = HelmBackend::default();
    let provider = test_provider("akeyless");
    let resource = test_resource("static_secret");

    let artifacts = backend
        .generate_resource(&resource, &provider)
        .unwrap();

    assert!(
        !artifacts.iter().any(|a| a.path.contains("fluxcd/")),
        "default backend should not produce fluxcd artifacts"
    );
}

#[test]
fn backend_with_fluxcd_provider_produces_kustomization() {
    let backend = HelmBackend::builder()
        .fluxcd_config(FluxCdConfig::default())
        .build();
    let provider = test_provider("akeyless");
    let resources = vec![
        test_resource("auth_method_api_key"),
        test_resource("static_secret"),
    ];

    let artifacts = backend
        .generate_provider(&provider, &resources, &[])
        .unwrap();

    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].path, "fluxcd/kustomization.yaml");
    assert!(artifacts[0].content.contains("kind: Kustomization"));
    assert!(artifacts[0]
        .content
        .contains("helmrelease-auth-method-api-key.yaml"));
    assert!(artifacts[0]
        .content
        .contains("helmrelease-static-secret.yaml"));
}

#[test]
fn backend_without_fluxcd_provider_returns_empty() {
    let backend = HelmBackend::default();
    let provider = test_provider("akeyless");
    let resources = vec![test_resource("static_secret")];

    let artifacts = backend
        .generate_provider(&provider, &resources, &[])
        .unwrap();

    assert!(artifacts.is_empty());
}

#[test]
fn fluxcd_generator_trait_is_object_safe_integration() {
    let generator: Box<dyn FluxCdGenerator> = Box::new(DefaultFluxCdGenerator {
        config: FluxCdConfig::default(),
    });
    let resource = test_resource("test");
    let output = generator.generate(&resource, "akeyless");
    assert!(output.contains("kind: HelmRelease"));
}

#[test]
fn fluxcd_custom_config_through_builder() {
    let config = FluxCdConfig {
        namespace: "prod-system".into(),
        source_name: "my-charts".into(),
        source_namespace: "gitops".into(),
        source_kind: "HelmRepository".into(),
        interval: "10m".into(),
        chart_interval: "30m".into(),
        retries: 5,
    };
    let backend = HelmBackend::builder().fluxcd_config(config).build();
    let provider = test_provider("test");
    let resource = test_resource("my_app");

    let artifacts = backend
        .generate_resource(&resource, &provider)
        .unwrap();

    let fluxcd = artifacts
        .iter()
        .find(|a| a.path.contains("fluxcd/"))
        .expect("should produce fluxcd artifact");
    assert!(fluxcd.content.contains("namespace: prod-system"));
    assert!(fluxcd.content.contains("name: my-charts"));
    assert!(fluxcd.content.contains("namespace: gitops"));
    assert!(fluxcd.content.contains("kind: HelmRepository"));
    assert!(fluxcd.content.contains("interval: 10m"));
    assert!(fluxcd.content.contains("interval: 30m"));
    assert!(fluxcd.content.contains("retries: 5"));
}

#[test]
fn fluxcd_end_to_end_write_to_disk() {
    let backend = HelmBackend::builder()
        .fluxcd_config(FluxCdConfig::default())
        .build();
    let provider = test_provider("akeyless");
    let resources = vec![
        test_resource("auth_method_api_key"),
        test_resource("static_secret"),
    ];

    let tmpdir = tempfile::TempDir::new().unwrap();

    // Generate resource artifacts (includes fluxcd helmrelease files)
    for resource in &resources {
        let artifacts = backend
            .generate_resource(resource, &provider)
            .unwrap();
        for artifact in &artifacts {
            let path = tmpdir.path().join(&artifact.path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, &artifact.content).unwrap();
        }
    }

    // Generate provider artifacts (kustomization)
    let provider_artifacts = backend
        .generate_provider(&provider, &resources, &[])
        .unwrap();
    for artifact in &provider_artifacts {
        let path = tmpdir.path().join(&artifact.path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, &artifact.content).unwrap();
    }

    // Verify files on disk
    let fluxcd_dir = tmpdir.path().join("fluxcd");
    assert!(fluxcd_dir.join("helmrelease-auth-method-api-key.yaml").exists());
    assert!(fluxcd_dir.join("helmrelease-static-secret.yaml").exists());
    assert!(fluxcd_dir.join("kustomization.yaml").exists());

    // Verify kustomization references the helmrelease files
    let kustomization = fs::read_to_string(fluxcd_dir.join("kustomization.yaml")).unwrap();
    assert!(kustomization.contains("helmrelease-auth-method-api-key.yaml"));
    assert!(kustomization.contains("helmrelease-static-secret.yaml"));
}

// ── Ports and service in generated values ─────────────────────────────────

#[test]
fn generated_values_have_ports_section() {
    let resource = test_resource("ports_test");
    let yaml = generate_values_yaml(&resource);
    let parsed: helm_forge::ValuesYaml = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(parsed.ports.len(), 1);
    assert_eq!(parsed.ports[0].name, "http");
    assert_eq!(parsed.ports[0].container_port, 8080);
    assert_eq!(parsed.ports[0].protocol, "TCP");
}

#[test]
fn generated_values_have_service_section() {
    let resource = test_resource("svc_test");
    let yaml = generate_values_yaml(&resource);
    let parsed: helm_forge::ValuesYaml = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(parsed.service.service_type, "ClusterIP");
    assert_eq!(parsed.service.ports.len(), 1);
    assert_eq!(parsed.service.ports[0].port, 80);
    assert_eq!(parsed.service.ports[0].target_port, "http");
}

// ── Image repository default ─────────────────────────────────────────────

#[test]
fn default_image_repository_is_not_empty() {
    let resource = test_resource("img_test");
    let yaml = generate_values_yaml(&resource);
    let parsed: helm_forge::ValuesYaml = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(parsed.image.repository, "ghcr.io/pleme-io/placeholder");
    assert!(!parsed.image.repository.is_empty());
}

// ── Array type rendering with toJson ──────────────────────────────────────

#[test]
fn configmap_list_attr_uses_to_json_integration() {
    // test_resource has a "tags" attribute of List(String), non-sensitive
    let resource = test_resource("json_test");
    let tpl = generate_configmap_template(&resource);
    assert!(
        tpl.contains("| toJson | quote"),
        "list attribute must use toJson | quote in configmap"
    );
}

#[test]
fn configmap_string_attr_uses_plain_quote_integration() {
    let resource = test_resource("json_test");
    let tpl = generate_configmap_template(&resource);
    // The "name" attribute is String, should use plain quote
    let name_line = tpl
        .lines()
        .find(|l| l.contains(".Values.config.name"))
        .expect("should have name config line");
    assert!(
        name_line.contains("| quote") && !name_line.contains("toJson"),
        "string attribute must use plain | quote"
    );
}

// ── minLength in schema for required strings ──────────────────────────────

#[test]
fn schema_required_string_has_min_length_integration() {
    let mut resource = test_resource("minlen_integration");
    resource.attributes.clear();
    resource.attributes.push(
        TestAttributeBuilder::new("name", IacType::String)
            .required()
            .build(),
    );
    resource.attributes.push(
        TestAttributeBuilder::new("optional_field", IacType::String)
            .build(),
    );

    let schema_str = generate_values_schema(&resource);
    let schema: Value = serde_json::from_str(&schema_str).unwrap();
    let config_props = &schema["properties"]["config"]["properties"];

    assert_eq!(
        config_props["name"]["minLength"], 1,
        "required string must have minLength: 1"
    );
    assert!(
        config_props["optional_field"].get("minLength").is_none(),
        "optional string must not have minLength"
    );
}

// ── Ports and service schema integration ──────────────────────────────────

#[test]
fn schema_has_ports_and_service_sections() {
    let resource = test_resource("schema_ports");
    let schema_str = generate_values_schema(&resource);
    let schema: Value = serde_json::from_str(&schema_str).unwrap();
    let props = schema["properties"].as_object().unwrap();

    assert!(props.contains_key("ports"), "schema missing ports");
    assert!(props.contains_key("service"), "schema missing service");

    // ports is array
    assert_eq!(props["ports"]["type"], "array");

    // service is object with type enum and ports
    assert_eq!(props["service"]["type"], "object");
    let svc_props = props["service"]["properties"].as_object().unwrap();
    assert!(svc_props.contains_key("type"));
    assert!(svc_props.contains_key("ports"));
}
