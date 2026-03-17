use iac_forge::backend::{
    ArtifactKind, Backend, GeneratedArtifact, NamingConvention,
};
use iac_forge::ir::{IacDataSource, IacProvider, IacResource};
use iac_forge::IacForgeError;

use crate::chart_gen::generate_chart_yaml;
use crate::naming::HelmNaming;
use crate::schema_gen::generate_values_schema;
use crate::template_gen::{
    generate_configmap_template, generate_deployment_template, generate_helpers_tpl,
    generate_networkpolicy_template, generate_secret_template, generate_service_template,
    generate_serviceaccount_template, generate_servicemonitor_template,
};
use crate::test_gen::generate_deployment_test;
use crate::values_gen::generate_values_yaml;

/// Helm chart generator backend.
///
/// For each `IacResource`, generates a complete Helm chart that delegates
/// to pleme-lib named templates. Sensitive attributes are placed in Secrets,
/// non-sensitive in ConfigMaps.
pub struct HelmBackend;

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
            content: generate_chart_yaml(resource, &provider.name),
            kind: ArtifactKind::Resource,
        });

        // values.yaml
        artifacts.push(GeneratedArtifact {
            path: format!("{base}/values.yaml"),
            content: generate_values_yaml(resource),
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

        // templates/deployment.yaml
        artifacts.push(GeneratedArtifact {
            path: format!("{base}/templates/deployment.yaml"),
            content: generate_deployment_template(),
            kind: ArtifactKind::Resource,
        });

        // templates/service.yaml
        artifacts.push(GeneratedArtifact {
            path: format!("{base}/templates/service.yaml"),
            content: generate_service_template(),
            kind: ArtifactKind::Resource,
        });

        // templates/serviceaccount.yaml
        artifacts.push(GeneratedArtifact {
            path: format!("{base}/templates/serviceaccount.yaml"),
            content: generate_serviceaccount_template(),
            kind: ArtifactKind::Resource,
        });

        // templates/servicemonitor.yaml
        artifacts.push(GeneratedArtifact {
            path: format!("{base}/templates/servicemonitor.yaml"),
            content: generate_servicemonitor_template(),
            kind: ArtifactKind::Resource,
        });

        // templates/networkpolicy.yaml
        artifacts.push(GeneratedArtifact {
            path: format!("{base}/templates/networkpolicy.yaml"),
            content: generate_networkpolicy_template(),
            kind: ArtifactKind::Resource,
        });

        // templates/configmap.yaml (only if non-sensitive attrs exist)
        let configmap = generate_configmap_template(resource);
        if !configmap.is_empty() {
            artifacts.push(GeneratedArtifact {
                path: format!("{base}/templates/configmap.yaml"),
                content: configmap,
                kind: ArtifactKind::Resource,
            });
        }

        // templates/secret.yaml (only if sensitive attrs exist)
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
        // Helm charts don't have a direct data source concept
        Ok(Vec::new())
    }

    fn generate_provider(
        &self,
        _provider: &IacProvider,
        _resources: &[IacResource],
        _data_sources: &[IacDataSource],
    ) -> Result<Vec<GeneratedArtifact>, IacForgeError> {
        // No provider-level artifacts needed for Helm
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
    use iac_forge::testing::{test_provider, test_resource};

    #[test]
    fn platform_is_helm() {
        let backend = HelmBackend;
        assert_eq!(backend.platform(), "helm");
    }

    #[test]
    fn generate_resource_produces_artifacts() {
        let backend = HelmBackend;
        let provider = test_provider("akeyless");
        let resource = test_resource("static_secret");

        let artifacts = backend.generate_resource(&resource, &provider).unwrap();

        // Should produce Chart.yaml, values.yaml, values.schema.json,
        // _helpers.tpl, deployment, service, serviceaccount, servicemonitor,
        // networkpolicy, configmap, secret
        assert!(artifacts.len() >= 9);

        let paths: Vec<&str> = artifacts.iter().map(|a| a.path.as_str()).collect();
        assert!(paths.contains(&"charts/static-secret/Chart.yaml"));
        assert!(paths.contains(&"charts/static-secret/values.yaml"));
        assert!(paths.contains(&"charts/static-secret/values.schema.json"));
        assert!(paths.contains(&"charts/static-secret/templates/_helpers.tpl"));
        assert!(paths.contains(&"charts/static-secret/templates/deployment.yaml"));
    }

    #[test]
    fn generate_test_produces_test_file() {
        let backend = HelmBackend;
        let provider = test_provider("akeyless");
        let resource = test_resource("static_secret");

        let artifacts = backend.generate_test(&resource, &provider).unwrap();
        assert_eq!(artifacts.len(), 1);
        assert!(artifacts[0].path.contains("tests/deployment_test.yaml"));
    }

    #[test]
    fn data_source_returns_empty() {
        let backend = HelmBackend;
        let provider = test_provider("akeyless");
        let ds = iac_forge::testing::test_data_source("test_ds");
        let artifacts = backend.generate_data_source(&ds, &provider).unwrap();
        assert!(artifacts.is_empty());
    }
}
