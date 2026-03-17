use iac_forge::backend::{
    ArtifactKind, Backend, GeneratedArtifact, NamingConvention,
};
use iac_forge::ir::{IacDataSource, IacProvider, IacResource};
use iac_forge::IacForgeError;

use crate::chart_gen::generate_chart_yaml_with_config;
use crate::config::HelmConfig;
use crate::naming::HelmNaming;
use crate::schema_gen::generate_values_schema;
use crate::template_gen::{
    generate_configmap_template, generate_deployment_template, generate_helpers_tpl,
    generate_hpa_template, generate_networkpolicy_template, generate_pdb_template,
    generate_podmonitor_template, generate_secret_template, generate_service_template,
    generate_serviceaccount_template, generate_servicemonitor_template,
};
use crate::test_gen::generate_deployment_test;
use crate::values_gen::generate_values_yaml_with_config;

/// Helm chart generator backend.
///
/// For each [`IacResource`], generates a complete Helm chart that delegates
/// to pleme-lib named templates. Sensitive attributes are placed in Secrets,
/// non-sensitive in ConfigMaps.
///
/// Use [`HelmBackend::default()`] for standard pleme-io conventions, or
/// [`HelmBackend::with_config()`] to customise chart metadata and defaults.
#[derive(Debug, Clone)]
pub struct HelmBackend {
    config: HelmConfig,
}

impl Default for HelmBackend {
    fn default() -> Self {
        Self {
            config: HelmConfig::default(),
        }
    }
}

impl HelmBackend {
    /// Create a backend with custom configuration.
    #[must_use]
    pub fn with_config(config: HelmConfig) -> Self {
        Self { config }
    }

    /// Return a reference to the active configuration.
    #[must_use]
    pub fn config(&self) -> &HelmConfig {
        &self.config
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

        // Chart.yaml
        artifacts.push(GeneratedArtifact {
            path: format!("{base}/Chart.yaml"),
            content: generate_chart_yaml_with_config(resource, &provider.name, &self.config),
            kind: ArtifactKind::Resource,
        });

        // values.yaml
        artifacts.push(GeneratedArtifact {
            path: format!("{base}/values.yaml"),
            content: generate_values_yaml_with_config(resource, &self.config),
            kind: ArtifactKind::Resource,
        });

        // values.schema.json
        artifacts.push(GeneratedArtifact {
            path: format!("{base}/values.schema.json"),
            content: generate_values_schema(resource),
            kind: ArtifactKind::Schema,
        });

        // templates/_helpers.tpl
        artifacts.push(GeneratedArtifact {
            path: format!("{base}/templates/_helpers.tpl"),
            content: generate_helpers_tpl(resource),
            kind: ArtifactKind::Resource,
        });

        // All pleme-lib delegate templates
        let delegates = [
            ("deployment.yaml", generate_deployment_template()),
            ("service.yaml", generate_service_template()),
            ("serviceaccount.yaml", generate_serviceaccount_template()),
            ("servicemonitor.yaml", generate_servicemonitor_template()),
            ("networkpolicy.yaml", generate_networkpolicy_template()),
            ("pdb.yaml", generate_pdb_template()),
            ("hpa.yaml", generate_hpa_template()),
            ("podmonitor.yaml", generate_podmonitor_template()),
        ];

        for (file, content) in delegates {
            artifacts.push(GeneratedArtifact {
                path: format!("{base}/templates/{file}"),
                content,
                kind: ArtifactKind::Resource,
            });
        }

        // Conditional templates: configmap (non-sensitive) and secret (sensitive)
        let configmap = generate_configmap_template(resource);
        if !configmap.is_empty() {
            artifacts.push(GeneratedArtifact {
                path: format!("{base}/templates/configmap.yaml"),
                content: configmap,
                kind: ArtifactKind::Resource,
            });
        }

        let secret = generate_secret_template(resource);
        if !secret.is_empty() {
            artifacts.push(GeneratedArtifact {
                path: format!("{base}/templates/secret.yaml"),
                content: secret,
                kind: ArtifactKind::Resource,
            });
        }

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
            content: generate_deployment_test(resource),
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
        // Core files
        assert!(paths.contains(&"charts/static-secret/Chart.yaml"));
        assert!(paths.contains(&"charts/static-secret/values.yaml"));
        assert!(paths.contains(&"charts/static-secret/values.schema.json"));
        assert!(paths.contains(&"charts/static-secret/templates/_helpers.tpl"));
        // All pleme-lib delegates
        assert!(paths.contains(&"charts/static-secret/templates/deployment.yaml"));
        assert!(paths.contains(&"charts/static-secret/templates/service.yaml"));
        assert!(paths.contains(&"charts/static-secret/templates/serviceaccount.yaml"));
        assert!(paths.contains(&"charts/static-secret/templates/servicemonitor.yaml"));
        assert!(paths.contains(&"charts/static-secret/templates/networkpolicy.yaml"));
        assert!(paths.contains(&"charts/static-secret/templates/pdb.yaml"));
        assert!(paths.contains(&"charts/static-secret/templates/hpa.yaml"));
        assert!(paths.contains(&"charts/static-secret/templates/podmonitor.yaml"));
        // Conditional
        assert!(paths.contains(&"charts/static-secret/templates/configmap.yaml"));
        assert!(paths.contains(&"charts/static-secret/templates/secret.yaml"));
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
}
