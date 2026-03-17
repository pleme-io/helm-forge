use iac_forge::{IacAttribute, IacResource, IacType};

/// Generate a `values.yaml` for a resource.
///
/// Non-sensitive attributes go under a top-level `config` key.
/// Sensitive attributes go under a top-level `secrets` key.
/// Standard pleme-lib values (image, resources, etc.) are included.
#[must_use]
pub fn generate_values_yaml(resource: &IacResource) -> String {
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
    let config_attrs: Vec<&IacAttribute> = resource
        .attributes
        .iter()
        .filter(|a| !a.sensitive && !a.computed)
        .collect();

    if !config_attrs.is_empty() {
        lines.push("# Resource configuration (non-sensitive)".into());
        lines.push("config:".into());
        for attr in &config_attrs {
            let comment = if attr.description.is_empty() {
                String::new()
            } else {
                format!("  # {}", attr.description)
            };
            if !comment.is_empty() {
                lines.push(comment);
            }
            lines.push(format!("  {}: {}", attr.canonical_name, default_yaml_value(&attr.iac_type)));
        }
        lines.push(String::new());
    }

    // Sensitive attributes
    let secret_attrs: Vec<&IacAttribute> = resource
        .attributes
        .iter()
        .filter(|a| a.sensitive && !a.computed)
        .collect();

    if !secret_attrs.is_empty() {
        lines.push("# Sensitive values (stored in Secret)".into());
        lines.push("secrets:".into());
        for attr in &secret_attrs {
            let comment = if attr.description.is_empty() {
                String::new()
            } else {
                format!("  # {}", attr.description)
            };
            if !comment.is_empty() {
                lines.push(comment);
            }
            lines.push(format!("  {}: \"\"", attr.canonical_name));
        }
        lines.push(String::new());
    }

    // Standard pleme-lib values
    lines.push("resources:".into());
    lines.push("  requests:".into());
    lines.push("    cpu: 50m".into());
    lines.push("    memory: 64Mi".into());
    lines.push("  limits:".into());
    lines.push("    cpu: 200m".into());
    lines.push("    memory: 256Mi".into());
    lines.push(String::new());

    lines.push("monitoring:".into());
    lines.push("  enabled: true".into());
    lines.push(String::new());

    lines.push("networkPolicy:".into());
    lines.push("  enabled: true".into());
    lines.push(String::new());

    lines.join("\n")
}

fn default_yaml_value(iac_type: &IacType) -> String {
    match iac_type {
        IacType::String => "\"\"".into(),
        IacType::Integer => "0".into(),
        IacType::Float => "0.0".into(),
        IacType::Boolean => "false".into(),
        IacType::List(_) | IacType::Set(_) => "[]".into(),
        IacType::Map(_) => "{}".into(),
        IacType::Object { .. } => "{}".into(),
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
    }
}
