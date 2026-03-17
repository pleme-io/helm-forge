use iac_forge::{IacAttribute, IacResource};
use serde_json::{Map, Value};

use crate::type_map::iac_type_to_json_schema;

/// Generate a `values.schema.json` for a resource.
///
/// Produces a JSON Schema that validates the chart's `values.yaml`.
/// Config (non-sensitive) and secrets (sensitive) attributes are separated.
#[must_use]
pub fn generate_values_schema(resource: &IacResource) -> String {
    let mut root = Map::new();
    root.insert(
        "$schema".into(),
        Value::String("https://json-schema.org/draft/2020-12/schema".into()),
    );
    root.insert("type".into(), Value::String("object".into()));
    root.insert(
        "title".into(),
        Value::String(format!("{} values", resource.name)),
    );

    let mut properties = Map::new();

    // Config section (non-sensitive)
    let config_attrs: Vec<&IacAttribute> = resource
        .attributes
        .iter()
        .filter(|a| !a.sensitive && !a.computed)
        .collect();

    if !config_attrs.is_empty() {
        properties.insert("config".into(), build_section_schema(&config_attrs));
    }

    // Secrets section (sensitive)
    let secret_attrs: Vec<&IacAttribute> = resource
        .attributes
        .iter()
        .filter(|a| a.sensitive && !a.computed)
        .collect();

    if !secret_attrs.is_empty() {
        properties.insert("secrets".into(), build_section_schema(&secret_attrs));
    }

    root.insert("properties".into(), Value::Object(properties));

    serde_json::to_string_pretty(&Value::Object(root)).expect("JSON serialization cannot fail")
}

fn build_section_schema(attrs: &[&IacAttribute]) -> Value {
    let mut props = Map::new();
    let mut required = Vec::new();

    for attr in attrs {
        let mut field_schema = iac_type_to_json_schema(&attr.iac_type);

        // Add description if present
        if !attr.description.is_empty() {
            if let Value::Object(ref mut obj) = field_schema {
                obj.insert(
                    "description".into(),
                    Value::String(attr.description.clone()),
                );
            }
        }

        // Add default value if present
        if let Some(ref default) = attr.default_value {
            if let Value::Object(ref mut obj) = field_schema {
                obj.insert("default".into(), default.clone());
            }
        }

        props.insert(attr.canonical_name.clone(), field_schema);

        if attr.required {
            required.push(Value::String(attr.canonical_name.clone()));
        }
    }

    let mut section = Map::new();
    section.insert("type".into(), Value::String("object".into()));
    section.insert("properties".into(), Value::Object(props));
    if !required.is_empty() {
        section.insert("required".into(), Value::Array(required));
    }
    Value::Object(section)
}

#[cfg(test)]
mod tests {
    use super::*;
    use iac_forge::testing::test_resource;

    #[test]
    fn generates_valid_json_schema() {
        let resource = test_resource("static_secret");
        let schema_str = generate_values_schema(&resource);
        let schema: Value = serde_json::from_str(&schema_str).expect("must be valid JSON");
        assert_eq!(schema["type"], "object");
        assert!(schema["$schema"].is_string());
        assert!(schema["properties"].is_object());
    }

    #[test]
    fn separates_config_and_secrets() {
        let resource = test_resource("static_secret");
        let schema_str = generate_values_schema(&resource);
        let schema: Value = serde_json::from_str(&schema_str).unwrap();
        let props = schema["properties"].as_object().unwrap();
        assert!(props.contains_key("config"));
        assert!(props.contains_key("secrets"));
    }
}
