use iac_forge::{IacResource, to_kebab_case};

use crate::config::HelmConfig;

/// Generate a `Chart.yaml` for a resource using the given configuration.
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
    let chart_name = to_kebab_case(&resource.name);
    let description = if resource.description.is_empty() {
        format!("Helm chart for {}", resource.name)
    } else {
        resource.description.clone()
    };

    format!(
        r#"apiVersion: v2
name: {chart_name}
description: {description}
type: application
version: {chart_version}
appVersion: "{app_version}"

dependencies:
  - name: {lib_name}
    version: "{lib_version}"
    repository: "{lib_repo}"

annotations:
  pleme.io/provider: {provider_name}
  pleme.io/category: {category}
  pleme.io/generated: "true"
"#,
        chart_version = config.default_chart_version,
        app_version = config.default_app_version,
        lib_name = config.lib_chart_name,
        lib_version = config.lib_chart_version,
        lib_repo = config.lib_chart_repository,
        category = resource.category,
    )
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
        assert!(yaml.contains("pleme.io/generated: \"true\""));
    }

    #[test]
    fn respects_custom_config() {
        let resource = test_resource("static_secret");
        let config = HelmConfig {
            lib_chart_version: "~0.5.0".into(),
            default_chart_version: "1.2.3".into(),
            ..HelmConfig::default()
        };
        let yaml = generate_chart_yaml_with_config(&resource, "akeyless", &config);
        assert!(yaml.contains("version: 1.2.3"));
        assert!(yaml.contains("\"~0.5.0\""));
    }
}
