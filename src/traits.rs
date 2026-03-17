//! Generator traits for mockability and substitution.
//!
//! Each generation step is behind a trait so consumers can:
//! - Mock individual generators in tests
//! - Replace generators with custom implementations
//! - Inspect generation plans without producing output

use iac_forge::ir::{IacAttribute, IacResource};

use crate::config::HelmConfig;

// ── Attribute filtering ─────────────────────────────────────────────────────

/// Filter resource attributes into config (non-sensitive) and secret (sensitive) groups.
pub trait AttributeFilter: std::fmt::Debug + Send + Sync {
    /// Attributes that should appear in ConfigMap and `config:` values section.
    fn config_attributes<'a>(&self, resource: &'a IacResource) -> Vec<&'a IacAttribute>;
    /// Attributes that should appear in Secret and `secrets:` values section.
    fn secret_attributes<'a>(&self, resource: &'a IacResource) -> Vec<&'a IacAttribute>;
}

/// Default filter: non-sensitive + non-computed → config, sensitive + non-computed → secret.
#[derive(Debug, Clone, Copy)]
pub struct DefaultAttributeFilter;

impl AttributeFilter for DefaultAttributeFilter {
    fn config_attributes<'a>(&self, resource: &'a IacResource) -> Vec<&'a IacAttribute> {
        resource
            .attributes
            .iter()
            .filter(|a| !a.sensitive && !a.computed)
            .collect()
    }

    fn secret_attributes<'a>(&self, resource: &'a IacResource) -> Vec<&'a IacAttribute> {
        resource
            .attributes
            .iter()
            .filter(|a| a.sensitive && !a.computed)
            .collect()
    }
}

// ── Generator traits ────────────────────────────────────────────────────────

/// Generates `Chart.yaml` content.
pub trait ChartGenerator: std::fmt::Debug + Send + Sync {
    fn generate(&self, resource: &IacResource, provider_name: &str) -> String;
}

/// Generates `values.yaml` content.
pub trait ValuesGenerator: std::fmt::Debug + Send + Sync {
    fn generate(&self, resource: &IacResource) -> String;
}

/// Generates `values.schema.json` content.
pub trait SchemaGenerator: std::fmt::Debug + Send + Sync {
    fn generate(&self, resource: &IacResource) -> String;
}

/// Generates Helm template files.
pub trait TemplateGenerator: std::fmt::Debug + Send + Sync {
    /// All delegate templates (deployment.yaml, service.yaml, etc.).
    fn delegate_templates(&self) -> Vec<(&'static str, String)>;
    /// `_helpers.tpl` for chart-specific helper definitions.
    fn helpers(&self, resource: &IacResource) -> String;
    /// ConfigMap template (empty string if no config attributes).
    fn configmap(&self, resource: &IacResource) -> String;
    /// Secret template (empty string if no secret attributes).
    fn secret(&self, resource: &IacResource) -> String;
    /// PrometheusRule template for alerting rules.
    fn prometheusrule(&self, resource: &IacResource) -> String;
}

/// Generates helm-unittest test files.
pub trait TestFileGenerator: std::fmt::Debug + Send + Sync {
    fn generate(&self, resource: &IacResource) -> String;
}

// ── Default implementations ─────────────────────────────────────────────────

/// Default `Chart.yaml` generator using `HelmConfig`.
#[derive(Debug, Clone)]
pub struct DefaultChartGenerator {
    pub config: HelmConfig,
}

impl ChartGenerator for DefaultChartGenerator {
    fn generate(&self, resource: &IacResource, provider_name: &str) -> String {
        crate::chart_gen::generate_chart_yaml_with_config(resource, provider_name, &self.config)
    }
}

/// Default `values.yaml` generator using `HelmConfig`.
#[derive(Debug, Clone)]
pub struct DefaultValuesGenerator {
    pub config: HelmConfig,
}

impl ValuesGenerator for DefaultValuesGenerator {
    fn generate(&self, resource: &IacResource) -> String {
        crate::values_gen::generate_values_yaml_with_config(resource, &self.config)
    }
}

/// Default `values.schema.json` generator.
#[derive(Debug, Clone, Copy)]
pub struct DefaultSchemaGenerator;

impl SchemaGenerator for DefaultSchemaGenerator {
    fn generate(&self, resource: &IacResource) -> String {
        crate::schema_gen::generate_values_schema(resource)
    }
}

/// Default Helm template generator delegating to pleme-lib.
#[derive(Debug, Clone, Copy)]
pub struct DefaultTemplateGenerator;

impl TemplateGenerator for DefaultTemplateGenerator {
    fn delegate_templates(&self) -> Vec<(&'static str, String)> {
        use crate::template_gen::*;
        vec![
            ("deployment.yaml", generate_deployment_template()),
            ("service.yaml", generate_service_template()),
            ("serviceaccount.yaml", generate_serviceaccount_template()),
            ("servicemonitor.yaml", generate_servicemonitor_template()),
            ("networkpolicy.yaml", generate_networkpolicy_template()),
            ("pdb.yaml", generate_pdb_template()),
            ("hpa.yaml", generate_hpa_template()),
            ("podmonitor.yaml", generate_podmonitor_template()),
        ]
    }

    fn helpers(&self, resource: &IacResource) -> String {
        crate::template_gen::generate_helpers_tpl(resource)
    }

    fn configmap(&self, resource: &IacResource) -> String {
        crate::template_gen::generate_configmap_template(resource)
    }

    fn secret(&self, resource: &IacResource) -> String {
        crate::template_gen::generate_secret_template(resource)
    }

    fn prometheusrule(&self, resource: &IacResource) -> String {
        crate::template_gen::generate_prometheusrule_template(resource)
    }
}

/// Default helm-unittest test generator.
#[derive(Debug, Clone, Copy)]
pub struct DefaultTestFileGenerator;

impl TestFileGenerator for DefaultTestFileGenerator {
    fn generate(&self, resource: &IacResource) -> String {
        crate::test_gen::generate_deployment_test(resource)
    }
}

// ── Generation pipeline (FSM) ───────────────────────────────────────────────

/// Stage in the chart generation pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationStage {
    /// Initial state — nothing generated yet.
    Init,
    /// Chart.yaml generated.
    ChartMetadata,
    /// values.yaml generated.
    Values,
    /// values.schema.json generated.
    Schema,
    /// All template files generated.
    Templates,
    /// Test files generated.
    Tests,
    /// Pipeline complete.
    Done,
}

impl GenerationStage {
    /// All stages in execution order.
    pub const ALL: &'static [Self] = &[
        Self::Init,
        Self::ChartMetadata,
        Self::Values,
        Self::Schema,
        Self::Templates,
        Self::Tests,
        Self::Done,
    ];

    /// Next stage in the pipeline, or `None` if already `Done`.
    #[must_use]
    pub fn next(self) -> Option<Self> {
        match self {
            Self::Init => Some(Self::ChartMetadata),
            Self::ChartMetadata => Some(Self::Values),
            Self::Values => Some(Self::Schema),
            Self::Schema => Some(Self::Templates),
            Self::Templates => Some(Self::Tests),
            Self::Tests => Some(Self::Done),
            Self::Done => None,
        }
    }

    /// Whether the pipeline is complete.
    #[must_use]
    pub fn is_done(self) -> bool {
        self == Self::Done
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iac_forge::testing::{test_resource, test_resource_with_type, TestAttributeBuilder};
    use iac_forge::IacType;

    // ── AttributeFilter ─────────────────────────────────────────────────

    #[test]
    fn default_filter_separates_config_and_secrets() {
        let resource = test_resource("test");
        let filter = DefaultAttributeFilter;

        let config = filter.config_attributes(&resource);
        let secrets = filter.secret_attributes(&resource);

        assert!(!config.is_empty());
        assert!(!secrets.is_empty());
        for c in &config {
            assert!(!c.sensitive, "config attr {} is sensitive", c.canonical_name);
        }
        for s in &secrets {
            assert!(s.sensitive, "secret attr {} is not sensitive", s.canonical_name);
        }
    }

    #[test]
    fn default_filter_excludes_computed() {
        let mut resource = test_resource_with_type("test", "computed_field", IacType::String);
        resource.attributes[0].computed = true;
        let filter = DefaultAttributeFilter;
        assert!(filter.config_attributes(&resource).is_empty());
        assert!(filter.secret_attributes(&resource).is_empty());
    }

    #[test]
    fn default_filter_empty_resource() {
        let mut resource = test_resource("empty");
        resource.attributes.clear();
        let filter = DefaultAttributeFilter;
        assert!(filter.config_attributes(&resource).is_empty());
        assert!(filter.secret_attributes(&resource).is_empty());
    }

    #[test]
    fn default_filter_only_sensitive() {
        let mut resource = test_resource_with_type("sens", "secret_val", IacType::String);
        resource.attributes[0].sensitive = true;
        let filter = DefaultAttributeFilter;
        assert!(filter.config_attributes(&resource).is_empty());
        assert_eq!(filter.secret_attributes(&resource).len(), 1);
    }

    #[test]
    fn default_filter_only_non_sensitive() {
        let resource = test_resource_with_type("plain", "plain_val", IacType::String);
        let filter = DefaultAttributeFilter;
        assert_eq!(filter.config_attributes(&resource).len(), 1);
        assert!(filter.secret_attributes(&resource).is_empty());
    }

    #[test]
    fn default_filter_mixed_with_computed() {
        let mut resource = test_resource("mixed");
        resource.attributes.push(
            TestAttributeBuilder::new("output_id", IacType::String)
                .computed()
                .build(),
        );
        let filter = DefaultAttributeFilter;
        let config = filter.config_attributes(&resource);
        let secrets = filter.secret_attributes(&resource);
        assert!(!config.iter().any(|a| a.canonical_name == "output_id"));
        assert!(!secrets.iter().any(|a| a.canonical_name == "output_id"));
    }

    // ── Default generator implementations ───────────────────────────────

    #[test]
    fn default_chart_generator_produces_valid_output() {
        let g = DefaultChartGenerator {
            config: HelmConfig::default(),
        };
        let resource = test_resource("test_res");
        let output = g.generate(&resource, "akeyless");
        assert!(output.contains("apiVersion: v2"));
        assert!(output.contains("name: test-res"));
        assert!(output.contains("pleme-lib"));
    }

    #[test]
    fn default_values_generator_produces_valid_output() {
        let g = DefaultValuesGenerator {
            config: HelmConfig::default(),
        };
        let resource = test_resource("test_res");
        let output = g.generate(&resource);
        assert!(output.contains("image:"));
        assert!(output.contains("resources:"));
    }

    #[test]
    fn default_schema_generator_produces_valid_json() {
        let g = DefaultSchemaGenerator;
        let resource = test_resource("test_res");
        let output = g.generate(&resource);
        let _: serde_json::Value =
            serde_json::from_str(&output).expect("must produce valid JSON");
    }

    #[test]
    fn default_template_generator_delegates_match_pleme_lib() {
        let g = DefaultTemplateGenerator;
        let delegates = g.delegate_templates();
        assert!(delegates.len() >= 8);
        for (name, content) in &delegates {
            assert!(
                content.contains("pleme-lib."),
                "{name} missing pleme-lib reference"
            );
        }
    }

    #[test]
    fn default_template_generator_helpers() {
        let g = DefaultTemplateGenerator;
        let resource = test_resource("test_res");
        let helpers = g.helpers(&resource);
        assert!(helpers.contains("test-res.name"));
        assert!(helpers.contains("pleme-lib.name"));
    }

    #[test]
    fn default_template_generator_prometheusrule() {
        let g = DefaultTemplateGenerator;
        let resource = test_resource("test_res");
        let output = g.prometheusrule(&resource);
        assert!(output.contains("kind: PrometheusRule"));
        assert!(output.contains("monitoring.coreos.com/v1"));
        assert!(output.contains("test-res.fullname"));
    }

    #[test]
    fn default_test_file_generator_produces_test() {
        let g = DefaultTestFileGenerator;
        let resource = test_resource("test_res");
        let output = g.generate(&resource);
        assert!(output.contains("suite: test-res"));
    }

    // ── Mock implementations for testing ────────────────────────────────

    #[derive(Debug)]
    struct MockChartGenerator {
        output: String,
    }

    impl ChartGenerator for MockChartGenerator {
        fn generate(&self, _resource: &IacResource, _provider_name: &str) -> String {
            self.output.clone()
        }
    }

    #[test]
    fn mock_chart_generator_returns_custom_output() {
        let mock = MockChartGenerator {
            output: "mock-chart-yaml".into(),
        };
        let resource = test_resource("test");
        assert_eq!(mock.generate(&resource, "test"), "mock-chart-yaml");
    }

    #[derive(Debug)]
    struct MockAttributeFilter {
        config_names: Vec<String>,
    }

    impl AttributeFilter for MockAttributeFilter {
        fn config_attributes<'a>(&self, resource: &'a IacResource) -> Vec<&'a IacAttribute> {
            resource
                .attributes
                .iter()
                .filter(|a| self.config_names.contains(&a.canonical_name))
                .collect()
        }

        fn secret_attributes<'a>(&self, _resource: &'a IacResource) -> Vec<&'a IacAttribute> {
            vec![] // mock: no secrets
        }
    }

    #[test]
    fn mock_attribute_filter_overrides_defaults() {
        let mock = MockAttributeFilter {
            config_names: vec!["name".into()],
        };
        let resource = test_resource("test");
        let config = mock.config_attributes(&resource);
        assert_eq!(config.len(), 1);
        assert_eq!(config[0].canonical_name, "name");
        assert!(mock.secret_attributes(&resource).is_empty());
    }

    // ── GenerationStage FSM ─────────────────────────────────────────────

    #[test]
    fn stage_transitions_complete_pipeline() {
        let mut stage = GenerationStage::Init;
        let mut count = 0;
        while let Some(next) = stage.next() {
            stage = next;
            count += 1;
        }
        assert_eq!(stage, GenerationStage::Done);
        assert_eq!(count, 6); // Init→ChartMetadata→Values→Schema→Templates→Tests→Done
    }

    #[test]
    fn stage_done_has_no_next() {
        assert!(GenerationStage::Done.next().is_none());
    }

    #[test]
    fn stage_is_done() {
        assert!(!GenerationStage::Init.is_done());
        assert!(!GenerationStage::Templates.is_done());
        assert!(GenerationStage::Done.is_done());
    }

    #[test]
    fn all_stages_ordered() {
        assert_eq!(GenerationStage::ALL.len(), 7);
        assert_eq!(GenerationStage::ALL[0], GenerationStage::Init);
        assert_eq!(GenerationStage::ALL[6], GenerationStage::Done);
    }

    #[test]
    fn all_traits_are_object_safe() {
        // Verify every trait can be used as a trait object
        let _: Box<dyn AttributeFilter> = Box::new(DefaultAttributeFilter);
        let _: Box<dyn ChartGenerator> = Box::new(DefaultChartGenerator {
            config: HelmConfig::default(),
        });
        let _: Box<dyn ValuesGenerator> = Box::new(DefaultValuesGenerator {
            config: HelmConfig::default(),
        });
        let _: Box<dyn SchemaGenerator> = Box::new(DefaultSchemaGenerator);
        let _: Box<dyn TemplateGenerator> = Box::new(DefaultTemplateGenerator);
        let _: Box<dyn TestFileGenerator> = Box::new(DefaultTestFileGenerator);
    }
}
