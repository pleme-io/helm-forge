use iac_forge::backend::{
    ArtifactKind, Backend, GeneratedArtifact, NamingConvention,
};
use iac_forge::ir::{IacDataSource, IacProvider, IacResource};
use iac_forge::IacForgeError;

use crate::config::HelmConfig;
use crate::naming::HelmNaming;
use crate::traits::{
    AttributeFilter, ChartGenerator, DefaultAttributeFilter, DefaultChartGenerator,
    DefaultSchemaGenerator, DefaultTemplateGenerator, DefaultTestFileGenerator,
    DefaultValuesGenerator, GenerationStage, SchemaGenerator, TemplateGenerator,
    TestFileGenerator, ValuesGenerator,
};

/// Helm chart generator backend with pluggable generators.
///
/// Every generation step is behind a trait, enabling mock substitution in tests
/// and custom implementations for non-standard chart layouts.
///
/// Use [`HelmBackend::default()`] for standard pleme-io conventions, or
/// [`HelmBackend::builder()`] to inject custom generators.
#[derive(Debug)]
pub struct HelmBackend {
    chart_gen: Box<dyn ChartGenerator>,
    values_gen: Box<dyn ValuesGenerator>,
    schema_gen: Box<dyn SchemaGenerator>,
    template_gen: Box<dyn TemplateGenerator>,
    test_gen: Box<dyn TestFileGenerator>,
    filter: Box<dyn AttributeFilter>,
    config: HelmConfig,
}

impl Default for HelmBackend {
    fn default() -> Self {
        Self::with_config(HelmConfig::default())
    }
}

impl HelmBackend {
    /// Create a backend with custom configuration but default generators.
    #[must_use]
    pub fn with_config(config: HelmConfig) -> Self {
        Self {
            chart_gen: Box::new(DefaultChartGenerator {
                config: config.clone(),
            }),
            values_gen: Box::new(DefaultValuesGenerator {
                config: config.clone(),
            }),
            schema_gen: Box::new(DefaultSchemaGenerator),
            template_gen: Box::new(DefaultTemplateGenerator),
            test_gen: Box::new(DefaultTestFileGenerator),
            filter: Box::new(DefaultAttributeFilter),
            config,
        }
    }

    /// Build a backend with full control over every generator.
    #[must_use]
    pub fn builder() -> HelmBackendBuilder {
        HelmBackendBuilder::new()
    }

    /// Return a reference to the active configuration.
    #[must_use]
    pub fn config(&self) -> &HelmConfig {
        &self.config
    }

    /// Return a reference to the active attribute filter.
    #[must_use]
    pub fn filter(&self) -> &dyn AttributeFilter {
        &*self.filter
    }

    /// Return the current generation stage for a given resource (always `Init`
    /// since `HelmBackend` runs the full pipeline in one call).
    #[must_use]
    pub fn stage(&self) -> GenerationStage {
        GenerationStage::Init
    }
}

/// Builder for `HelmBackend` with dependency injection.
pub struct HelmBackendBuilder {
    config: HelmConfig,
    chart_gen: Option<Box<dyn ChartGenerator>>,
    values_gen: Option<Box<dyn ValuesGenerator>>,
    schema_gen: Option<Box<dyn SchemaGenerator>>,
    template_gen: Option<Box<dyn TemplateGenerator>>,
    test_gen: Option<Box<dyn TestFileGenerator>>,
    filter: Option<Box<dyn AttributeFilter>>,
}

impl HelmBackendBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: HelmConfig::default(),
            chart_gen: None,
            values_gen: None,
            schema_gen: None,
            template_gen: None,
            test_gen: None,
            filter: None,
        }
    }

    #[must_use]
    pub fn config(mut self, config: HelmConfig) -> Self {
        self.config = config;
        self
    }

    #[must_use]
    pub fn chart_generator(mut self, generator: Box<dyn ChartGenerator>) -> Self {
        self.chart_gen = Some(generator);
        self
    }

    #[must_use]
    pub fn values_generator(mut self, generator: Box<dyn ValuesGenerator>) -> Self {
        self.values_gen = Some(generator);
        self
    }

    #[must_use]
    pub fn schema_generator(mut self, generator: Box<dyn SchemaGenerator>) -> Self {
        self.schema_gen = Some(generator);
        self
    }

    #[must_use]
    pub fn template_generator(mut self, generator: Box<dyn TemplateGenerator>) -> Self {
        self.template_gen = Some(generator);
        self
    }

    #[must_use]
    pub fn test_generator(mut self, generator: Box<dyn TestFileGenerator>) -> Self {
        self.test_gen = Some(generator);
        self
    }

    #[must_use]
    pub fn attribute_filter(mut self, filter: Box<dyn AttributeFilter>) -> Self {
        self.filter = Some(filter);
        self
    }

    /// Build the `HelmBackend`. Uses default implementations for any generators
    /// not explicitly set.
    #[must_use]
    pub fn build(self) -> HelmBackend {
        let config = self.config;
        HelmBackend {
            chart_gen: self.chart_gen.unwrap_or_else(|| {
                Box::new(DefaultChartGenerator {
                    config: config.clone(),
                })
            }),
            values_gen: self.values_gen.unwrap_or_else(|| {
                Box::new(DefaultValuesGenerator {
                    config: config.clone(),
                })
            }),
            schema_gen: self.schema_gen.unwrap_or_else(|| Box::new(DefaultSchemaGenerator)),
            template_gen: self
                .template_gen
                .unwrap_or_else(|| Box::new(DefaultTemplateGenerator)),
            test_gen: self
                .test_gen
                .unwrap_or_else(|| Box::new(DefaultTestFileGenerator)),
            filter: self
                .filter
                .unwrap_or_else(|| Box::new(DefaultAttributeFilter)),
            config,
        }
    }
}

impl Default for HelmBackendBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for HelmBackend {
    fn platform(&self) -> &str {
        "helm"
    }

    fn generate_resource(
        &self,
        resource: &IacResource,
        provider: &IacProvider,
    ) -> Result<Vec<GeneratedArtifact>, IacForgeError> {
        let chart_name = iac_forge::to_kebab_case(&resource.name);
        let base = format!("charts/{chart_name}");

        let mut artifacts = Vec::new();
        let mut _stage = GenerationStage::Init;

        // Stage: ChartMetadata
        artifacts.push(GeneratedArtifact {
            path: format!("{base}/Chart.yaml"),
            content: self.chart_gen.generate(resource, &provider.name),
            kind: ArtifactKind::Resource,
        });
        _stage = GenerationStage::ChartMetadata;

        // Stage: Values
        artifacts.push(GeneratedArtifact {
            path: format!("{base}/values.yaml"),
            content: self.values_gen.generate(resource),
            kind: ArtifactKind::Resource,
        });
        _stage = GenerationStage::Values;

        // Stage: Schema
        artifacts.push(GeneratedArtifact {
            path: format!("{base}/values.schema.json"),
            content: self.schema_gen.generate(resource),
            kind: ArtifactKind::Schema,
        });
        _stage = GenerationStage::Schema;

        // Stage: Templates
        artifacts.push(GeneratedArtifact {
            path: format!("{base}/templates/_helpers.tpl"),
            content: self.template_gen.helpers(resource),
            kind: ArtifactKind::Resource,
        });

        for (file, content) in self.template_gen.delegate_templates() {
            artifacts.push(GeneratedArtifact {
                path: format!("{base}/templates/{file}"),
                content,
                kind: ArtifactKind::Resource,
            });
        }

        let configmap = self.template_gen.configmap(resource);
        if !configmap.is_empty() {
            artifacts.push(GeneratedArtifact {
                path: format!("{base}/templates/configmap.yaml"),
                content: configmap,
                kind: ArtifactKind::Resource,
            });
        }

        let secret = self.template_gen.secret(resource);
        if !secret.is_empty() {
            artifacts.push(GeneratedArtifact {
                path: format!("{base}/templates/secret.yaml"),
                content: secret,
                kind: ArtifactKind::Resource,
            });
        }
        _stage = GenerationStage::Templates;

        // Stage: Tests
        artifacts.push(GeneratedArtifact {
            path: format!("{base}/tests/deployment_test.yaml"),
            content: self.test_gen.generate(resource),
            kind: ArtifactKind::Test,
        });
        _stage = GenerationStage::Done;

        Ok(artifacts)
    }

    fn generate_data_source(
        &self,
        _ds: &IacDataSource,
        _provider: &IacProvider,
    ) -> Result<Vec<GeneratedArtifact>, IacForgeError> {
        Ok(Vec::new())
    }

    fn generate_provider(
        &self,
        _provider: &IacProvider,
        _resources: &[IacResource],
        _data_sources: &[IacDataSource],
    ) -> Result<Vec<GeneratedArtifact>, IacForgeError> {
        Ok(Vec::new())
    }

    fn generate_test(
        &self,
        resource: &IacResource,
        _provider: &IacProvider,
    ) -> Result<Vec<GeneratedArtifact>, IacForgeError> {
        let chart_name = iac_forge::to_kebab_case(&resource.name);
        let base = format!("charts/{chart_name}");

        Ok(vec![GeneratedArtifact {
            path: format!("{base}/tests/deployment_test.yaml"),
            content: self.test_gen.generate(resource),
            kind: ArtifactKind::Test,
        }])
    }

    fn naming(&self) -> &dyn NamingConvention {
        &HelmNaming
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iac_forge::testing::{test_data_source, test_provider, test_resource};

    #[test]
    fn platform_is_helm() {
        assert_eq!(HelmBackend::default().platform(), "helm");
    }

    #[test]
    fn default_backend_uses_default_config() {
        let backend = HelmBackend::default();
        assert_eq!(backend.config().lib_chart_name, "pleme-lib");
    }

    #[test]
    fn custom_config_propagates() {
        let backend = HelmBackend::with_config(HelmConfig {
            lib_chart_version: "~1.0.0".into(),
            ..HelmConfig::default()
        });
        assert_eq!(backend.config().lib_chart_version, "~1.0.0");
    }

    #[test]
    fn generate_resource_produces_all_artifacts() {
        let backend = HelmBackend::default();
        let provider = test_provider("akeyless");
        let resource = test_resource("static_secret");

        let artifacts = backend.generate_resource(&resource, &provider).unwrap();
        let paths: Vec<&str> = artifacts.iter().map(|a| a.path.as_str()).collect();

        assert!(paths.contains(&"charts/static-secret/Chart.yaml"));
        assert!(paths.contains(&"charts/static-secret/values.yaml"));
        assert!(paths.contains(&"charts/static-secret/values.schema.json"));
        assert!(paths.contains(&"charts/static-secret/templates/_helpers.tpl"));
        assert!(paths.contains(&"charts/static-secret/templates/deployment.yaml"));
        assert!(paths.contains(&"charts/static-secret/templates/service.yaml"));
        assert!(paths.contains(&"charts/static-secret/templates/serviceaccount.yaml"));
        assert!(paths.contains(&"charts/static-secret/templates/servicemonitor.yaml"));
        assert!(paths.contains(&"charts/static-secret/templates/networkpolicy.yaml"));
        assert!(paths.contains(&"charts/static-secret/templates/pdb.yaml"));
        assert!(paths.contains(&"charts/static-secret/templates/hpa.yaml"));
        assert!(paths.contains(&"charts/static-secret/templates/podmonitor.yaml"));
        assert!(paths.contains(&"charts/static-secret/templates/configmap.yaml"));
        assert!(paths.contains(&"charts/static-secret/templates/secret.yaml"));
        assert!(paths.contains(&"charts/static-secret/tests/deployment_test.yaml"));
    }

    #[test]
    fn generate_test_produces_test_file() {
        let backend = HelmBackend::default();
        let provider = test_provider("akeyless");
        let resource = test_resource("static_secret");
        let artifacts = backend.generate_test(&resource, &provider).unwrap();
        assert_eq!(artifacts.len(), 1);
        assert!(artifacts[0].path.ends_with("tests/deployment_test.yaml"));
    }

    #[test]
    fn data_source_returns_empty() {
        let backend = HelmBackend::default();
        let provider = test_provider("akeyless");
        let ds = test_data_source("test_ds");
        assert!(backend.generate_data_source(&ds, &provider).unwrap().is_empty());
    }

    #[test]
    fn provider_returns_empty() {
        let backend = HelmBackend::default();
        let provider = test_provider("akeyless");
        assert!(backend.generate_provider(&provider, &[], &[]).unwrap().is_empty());
    }

    // ── Mock injection via builder ──────────────────────────────────────

    #[derive(Debug)]
    struct MockChartGen(String);
    impl ChartGenerator for MockChartGen {
        fn generate(&self, _: &IacResource, _: &str) -> String {
            self.0.clone()
        }
    }

    #[derive(Debug)]
    struct MockValuesGen(String);
    impl ValuesGenerator for MockValuesGen {
        fn generate(&self, _: &IacResource) -> String {
            self.0.clone()
        }
    }

    #[test]
    fn builder_injects_mock_chart_generator() {
        let backend = HelmBackend::builder()
            .chart_generator(Box::new(MockChartGen("MOCK_CHART".into())))
            .build();

        let provider = test_provider("test");
        let resource = test_resource("test");
        let artifacts = backend.generate_resource(&resource, &provider).unwrap();

        let chart = artifacts.iter().find(|a| a.path.ends_with("Chart.yaml")).unwrap();
        assert_eq!(chart.content, "MOCK_CHART");
    }

    #[test]
    fn builder_injects_mock_values_generator() {
        let backend = HelmBackend::builder()
            .values_generator(Box::new(MockValuesGen("MOCK_VALUES".into())))
            .build();

        let provider = test_provider("test");
        let resource = test_resource("test");
        let artifacts = backend.generate_resource(&resource, &provider).unwrap();

        let values = artifacts.iter().find(|a| a.path.ends_with("values.yaml")).unwrap();
        assert_eq!(values.content, "MOCK_VALUES");
    }

    #[test]
    fn builder_mixes_mock_and_default() {
        let backend = HelmBackend::builder()
            .chart_generator(Box::new(MockChartGen("CUSTOM".into())))
            // all others default
            .build();

        let provider = test_provider("test");
        let resource = test_resource("test");
        let artifacts = backend.generate_resource(&resource, &provider).unwrap();

        // Chart is custom
        let chart = artifacts.iter().find(|a| a.path.ends_with("Chart.yaml")).unwrap();
        assert_eq!(chart.content, "CUSTOM");

        // Values is default
        let values = artifacts.iter().find(|a| a.path.ends_with("values.yaml")).unwrap();
        assert!(values.content.contains("image:"));
    }

    #[test]
    fn builder_default_produces_same_as_default_constructor() {
        let provider = test_provider("test");
        let resource = test_resource("test");

        let from_default = HelmBackend::default()
            .generate_resource(&resource, &provider)
            .unwrap();
        let from_builder = HelmBackend::builder()
            .build()
            .generate_resource(&resource, &provider)
            .unwrap();

        assert_eq!(from_default.len(), from_builder.len());
        for (a, b) in from_default.iter().zip(from_builder.iter()) {
            assert_eq!(a.path, b.path);
            assert_eq!(a.content, b.content);
        }
    }
}
