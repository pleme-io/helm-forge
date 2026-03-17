use std::collections::BTreeMap;

use iac_forge::{IacResource, to_kebab_case};

use crate::config::HelmConfig;
use crate::model::{ChartDependency, ChartYaml};

/// Generate a `Chart.yaml` for a resource using default configuration.
#[must_use]
pub fn generate_chart_yaml(resource: &IacResource, provider_name: &str) -> String {
    generate_chart_yaml_with_config(resource, provider_name, &HelmConfig::default())
}

/// Generate a `Chart.yaml` for a resource with explicit configuration.
#[must_use]
pub fn generate_chart_yaml_with_config(
    resource: &IacResource,
    provider_name: &str,
    config: &HelmConfig,
) -> String {
    let chart = build_chart_yaml(resource, provider_name, config);
    serde_yaml_ng::to_string(&chart).expect("ChartYaml serialization cannot fail")
}

/// Build a [`ChartYaml`] struct from a resource, provider, and config.
///
/// Exposed for consumers who want to inspect or modify the struct before
/// serializing.
#[must_use]
pub fn build_chart_yaml(
    resource: &IacResource,
    provider_name: &str,
    config: &HelmConfig,
) -> ChartYaml {
    let chart_name = to_kebab_case(&resource.name);
    let description = if resource.description.is_empty() {
        format!("Helm chart for {}", resource.name)
    } else {
        resource.description.clone()
    };

    let mut annotations = BTreeMap::new();
    annotations.insert("pleme.io/category".into(), resource.category.clone());
    annotations.insert("pleme.io/generated".into(), "true".into());
    annotations.insert("pleme.io/provider".into(), provider_name.into());

    ChartYaml {
        api_version: "v2".into(),
        name: chart_name,
        description,
        chart_type: "application".into(),
        version: config.default_chart_version.clone(),
        app_version: config.default_app_version.clone(),
        dependencies: vec![ChartDependency {
            name: config.lib_chart_name.clone(),
            version: config.lib_chart_version.clone(),
            repository: config.lib_chart_repository.clone(),
        }],
        annotations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iac_forge::testing::test_resource;

    #[test]
    fn generates_valid_chart_yaml() {
        let resource = test_resource("static_secret");
        let yaml = generate_chart_yaml(&resource, "akeyless");
        assert!(yaml.contains("name: static-secret"));
        assert!(yaml.contains("apiVersion: v2"));
        assert!(yaml.contains("type: application"));
        assert!(yaml.contains("pleme-lib"));
        assert!(yaml.contains("pleme.io/generated: 'true'"));
    }

    #[test]
    fn round_trips_through_serde() {
        let resource = test_resource("static_secret");
        let chart = build_chart_yaml(&resource, "akeyless", &HelmConfig::default());
        let yaml = serde_yaml_ng::to_string(&chart).unwrap();
        let parsed: ChartYaml = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(chart, parsed);
    }

    #[test]
    fn respects_custom_config() {
        let resource = test_resource("static_secret");
        let config = HelmConfig {
            lib_chart_version: "~0.5.0".into(),
            default_chart_version: "1.2.3".into(),
            ..HelmConfig::default()
        };
        let chart = build_chart_yaml(&resource, "akeyless", &config);
        assert_eq!(chart.version, "1.2.3");
        assert_eq!(chart.dependencies[0].version, "~0.5.0");
    }

    #[test]
    fn build_returns_inspectable_struct() {
        let resource = test_resource("static_secret");
        let chart = build_chart_yaml(&resource, "akeyless", &HelmConfig::default());
        assert_eq!(chart.api_version, "v2");
        assert_eq!(chart.name, "static-secret");
        assert_eq!(chart.chart_type, "application");
        assert_eq!(chart.dependencies.len(), 1);
        assert!(chart.annotations.contains_key("pleme.io/generated"));
    }
}
