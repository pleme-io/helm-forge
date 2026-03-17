use iac_forge::{IacResource, IacType};

use crate::config::HelmConfig;
use crate::traits::{AttributeFilter, DefaultAttributeFilter};

/// Generate a `values.yaml` for a resource with default configuration.
///
/// Non-sensitive attributes go under a top-level `config` key.
/// Sensitive attributes go under a top-level `secrets` key.
/// Standard pleme-lib values (image, resources, etc.) are included.
#[must_use]
pub fn generate_values_yaml(resource: &IacResource) -> String {
    generate_values_yaml_with_config(resource, &HelmConfig::default())
}

/// Generate a `values.yaml` for a resource with explicit configuration.
#[must_use]
pub fn generate_values_yaml_with_config(resource: &IacResource, config: &HelmConfig) -> String {
    let mut lines = Vec::new();

    lines.push(format!("# values.yaml for {}", resource.name));
    lines.push(String::new());

    // Image configuration
    lines.push("image:".into());
    lines.push("  repository: \"\"".into());
    lines.push("  tag: latest".into());
    lines.push("  pullPolicy: Always".into());
    lines.push(String::new());

    lines.push("replicaCount: 1".into());
    lines.push(String::new());

    // Non-sensitive config attributes
    let filter = DefaultAttributeFilter;
    let config_attrs = filter.config_attributes(resource);

    if !config_attrs.is_empty() {
        lines.push("# Resource configuration (non-sensitive)".into());
        lines.push("config:".into());
        for attr in &config_attrs {
            if !attr.description.is_empty() {
                lines.push(format!("  # {}", attr.description));
            }
            lines.push(format!(
                "  {}: {}",
                attr.canonical_name,
                default_yaml_value(&attr.iac_type)
            ));
        }
        lines.push(String::new());
    }

    // Sensitive attributes
    let secret_attrs = filter.secret_attributes(resource);

    if !secret_attrs.is_empty() {
        lines.push("# Sensitive values (stored in Secret)".into());
        lines.push("secrets:".into());
        for attr in &secret_attrs {
            if !attr.description.is_empty() {
                lines.push(format!("  # {}", attr.description));
            }
            lines.push(format!("  {}: \"\"", attr.canonical_name));
        }
        lines.push(String::new());
    }

    // Standard pleme-lib values (configurable resource defaults)
    lines.push("resources:".into());
    lines.push("  requests:".into());
    lines.push(format!("    cpu: {}", config.cpu_request));
    lines.push(format!("    memory: {}", config.memory_request));
    lines.push("  limits:".into());
    lines.push(format!("    cpu: {}", config.cpu_limit));
    lines.push(format!("    memory: {}", config.memory_limit));
    lines.push(String::new());

    lines.push("monitoring:".into());
    lines.push("  enabled: true".into());
    lines.push(String::new());

    lines.push("networkPolicy:".into());
    lines.push("  enabled: true".into());
    lines.push(String::new());

    lines.push("pdb:".into());
    lines.push("  enabled: false".into());
    lines.push(String::new());

    lines.push("autoscaling:".into());
    lines.push("  enabled: false".into());
    lines.push(String::new());

    lines.join("\n")
}

/// Map an `IacType` to a sensible YAML default value string.
#[must_use]
pub fn default_yaml_value(iac_type: &IacType) -> String {
    match iac_type {
        IacType::String => "\"\"".into(),
        IacType::Integer => "0".into(),
        IacType::Float => "0.0".into(),
        IacType::Boolean => "false".into(),
        IacType::List(_) | IacType::Set(_) => "[]".into(),
        IacType::Map(_) | IacType::Object { .. } => "{}".into(),
        IacType::Enum { values, .. } => {
            if let Some(first) = values.first() {
                format!("\"{first}\"")
            } else {
                "\"\"".into()
            }
        }
        IacType::Any => "\"\"".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iac_forge::testing::test_resource;

    #[test]
    fn generates_values_with_config_and_secrets() {
        let resource = test_resource("static_secret");
        let yaml = generate_values_yaml(&resource);
        assert!(yaml.contains("config:"));
        assert!(yaml.contains("secrets:"));
        assert!(yaml.contains("resources:"));
        assert!(yaml.contains("monitoring:"));
        assert!(yaml.contains("pdb:"));
        assert!(yaml.contains("autoscaling:"));
    }

    #[test]
    fn respects_custom_resource_limits() {
        let resource = test_resource("static_secret");
        let config = HelmConfig {
            cpu_request: "100m".into(),
            memory_request: "128Mi".into(),
            cpu_limit: "500m".into(),
            memory_limit: "512Mi".into(),
            ..HelmConfig::default()
        };
        let yaml = generate_values_yaml_with_config(&resource, &config);
        assert!(yaml.contains("cpu: 100m"));
        assert!(yaml.contains("memory: 128Mi"));
        assert!(yaml.contains("cpu: 500m"));
        assert!(yaml.contains("memory: 512Mi"));
    }
}
