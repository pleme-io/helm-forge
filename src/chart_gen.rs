use iac_forge::{IacResource, to_kebab_case};

/// Generate a `Chart.yaml` for a resource.
#[must_use]
pub fn generate_chart_yaml(resource: &IacResource, provider_name: &str) -> String {
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
version: 0.1.0
appVersion: "1.0.0"

dependencies:
  - name: pleme-lib
    version: "~0.4.0"
    repository: "file://../pleme-lib"

annotations:
  pleme.io/provider: {provider_name}
  pleme.io/category: {category}
  pleme.io/generated: "true"
"#,
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
}
