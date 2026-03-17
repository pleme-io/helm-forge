use iac_forge::{IacAttribute, IacResource};
use serde_json::{Map, Value};

use crate::traits::{AttributeFilter, DefaultAttributeFilter};
use crate::type_map::iac_type_to_json_schema;

/// Generate a `values.schema.json` for a resource.
///
/// Produces a JSON Schema that validates the chart's `values.yaml`.
/// Includes schemas for both resource-specific attributes (config/secrets)
/// and standard pleme-lib values (image, resources, monitoring, etc.).
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
    let filter = DefaultAttributeFilter;

    // Config section (non-sensitive)
    let config_attrs = filter.config_attributes(resource);
    if !config_attrs.is_empty() {
        properties.insert("config".into(), build_section_schema(&config_attrs));
    }

    // Secrets section (sensitive)
    let secret_attrs = filter.secret_attributes(resource);
    if !secret_attrs.is_empty() {
        properties.insert("secrets".into(), build_section_schema(&secret_attrs));
    }

    // Standard pleme-lib value schemas
    properties.insert("image".into(), image_schema());
    properties.insert("replicaCount".into(), integer_with_default(1));
    properties.insert("resources".into(), resources_schema());
    properties.insert("monitoring".into(), monitoring_schema());
    properties.insert("networkPolicy".into(), enabled_toggle_schema());
    properties.insert("pdb".into(), enabled_toggle_schema());
    properties.insert("autoscaling".into(), enabled_toggle_schema());

    root.insert("properties".into(), Value::Object(properties));

    serde_json::to_string_pretty(&Value::Object(root)).expect("JSON serialization cannot fail")
}

/// Build a schema object for a group of attributes.
fn build_section_schema(attrs: &[&IacAttribute]) -> Value {
    let mut props = Map::new();
    let mut required = Vec::new();

    for attr in attrs {
        let mut field_schema = iac_type_to_json_schema(&attr.iac_type);

        if !attr.description.is_empty() {
            if let Value::Object(ref mut obj) = field_schema {
                obj.insert(
                    "description".into(),
                    Value::String(attr.description.clone()),
                );
            }
        }

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

/// JSON Schema for the `image` block.
fn image_schema() -> Value {
    let mut props = Map::new();
    props.insert("repository".into(), string_schema());
    props.insert("tag".into(), string_schema());
    props.insert(
        "pullPolicy".into(),
        enum_schema(&["Always", "IfNotPresent", "Never"]),
    );

    let mut obj = Map::new();
    obj.insert("type".into(), Value::String("object".into()));
    obj.insert("properties".into(), Value::Object(props));
    Value::Object(obj)
}

/// JSON Schema for the `resources` block (requests/limits).
fn resources_schema() -> Value {
    let quantity = string_schema();

    let mut req_props = Map::new();
    req_props.insert("cpu".into(), quantity.clone());
    req_props.insert("memory".into(), quantity.clone());

    let mut lim_props = Map::new();
    lim_props.insert("cpu".into(), quantity.clone());
    lim_props.insert("memory".into(), quantity);

    let mut props = Map::new();
    props.insert("requests".into(), object_schema(req_props));
    props.insert("limits".into(), object_schema(lim_props));

    object_schema(props)
}

/// JSON Schema for the `monitoring` block with alerting, interval, port, and path.
fn monitoring_schema() -> Value {
    let mut alerting_props = Map::new();
    alerting_props.insert("enabled".into(), bool_schema());

    let mut props = Map::new();
    props.insert("enabled".into(), bool_schema());
    props.insert("alerting".into(), object_schema(alerting_props));
    props.insert("interval".into(), string_schema());
    props.insert("port".into(), string_schema());
    props.insert("path".into(), string_schema());
    object_schema(props)
}

/// JSON Schema for a simple `{ enabled: bool }` toggle.
fn enabled_toggle_schema() -> Value {
    let mut props = Map::new();
    props.insert("enabled".into(), bool_schema());
    object_schema(props)
}

fn string_schema() -> Value {
    let mut obj = Map::new();
    obj.insert("type".into(), Value::String("string".into()));
    Value::Object(obj)
}

fn bool_schema() -> Value {
    let mut obj = Map::new();
    obj.insert("type".into(), Value::String("boolean".into()));
    Value::Object(obj)
}

fn integer_with_default(n: i64) -> Value {
    let mut obj = Map::new();
    obj.insert("type".into(), Value::String("integer".into()));
    obj.insert("default".into(), Value::Number(n.into()));
    Value::Object(obj)
}

fn enum_schema(values: &[&str]) -> Value {
    let mut obj = Map::new();
    obj.insert("type".into(), Value::String("string".into()));
    obj.insert(
        "enum".into(),
        Value::Array(values.iter().map(|v| Value::String((*v).into())).collect()),
    );
    Value::Object(obj)
}

fn object_schema(properties: Map<String, Value>) -> Value {
    let mut obj = Map::new();
    obj.insert("type".into(), Value::String("object".into()));
    obj.insert("properties".into(), Value::Object(properties));
    Value::Object(obj)
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

    #[test]
    fn includes_pleme_lib_standard_schemas() {
        let resource = test_resource("static_secret");
        let schema_str = generate_values_schema(&resource);
        let schema: Value = serde_json::from_str(&schema_str).unwrap();
        let props = schema["properties"].as_object().unwrap();

        assert!(props.contains_key("image"), "missing image schema");
        assert!(props.contains_key("replicaCount"), "missing replicaCount");
        assert!(props.contains_key("resources"), "missing resources");
        assert!(props.contains_key("monitoring"), "missing monitoring");
        assert!(props.contains_key("networkPolicy"), "missing networkPolicy");
        assert!(props.contains_key("pdb"), "missing pdb");
        assert!(props.contains_key("autoscaling"), "missing autoscaling");
    }

    #[test]
    fn image_schema_has_pull_policy_enum() {
        let resource = test_resource("test_res");
        let schema_str = generate_values_schema(&resource);
        let schema: Value = serde_json::from_str(&schema_str).unwrap();

        let pull_policy = &schema["properties"]["image"]["properties"]["pullPolicy"];
        assert_eq!(pull_policy["type"], "string");
        let enum_vals = pull_policy["enum"].as_array().unwrap();
        assert_eq!(enum_vals.len(), 3);
    }

    #[test]
    fn resources_schema_has_requests_and_limits() {
        let resource = test_resource("test_res");
        let schema_str = generate_values_schema(&resource);
        let schema: Value = serde_json::from_str(&schema_str).unwrap();

        let res = &schema["properties"]["resources"]["properties"];
        assert!(res["requests"].is_object());
        assert!(res["limits"].is_object());
        assert!(res["requests"]["properties"]["cpu"].is_object());
        assert!(res["limits"]["properties"]["memory"].is_object());
    }

    #[test]
    fn monitoring_schema_has_alerting_and_fields() {
        let resource = test_resource("test_res");
        let schema_str = generate_values_schema(&resource);
        let schema: Value = serde_json::from_str(&schema_str).unwrap();

        let monitoring = &schema["properties"]["monitoring"]["properties"];
        assert!(monitoring["enabled"].is_object(), "missing monitoring.enabled");
        assert!(monitoring["alerting"].is_object(), "missing monitoring.alerting");
        assert!(monitoring["interval"].is_object(), "missing monitoring.interval");
        assert!(monitoring["port"].is_object(), "missing monitoring.port");
        assert!(monitoring["path"].is_object(), "missing monitoring.path");

        // Alerting has its own enabled field.
        let alerting = &monitoring["alerting"]["properties"];
        assert!(alerting["enabled"].is_object(), "missing alerting.enabled");
    }

    #[test]
    fn replica_count_has_integer_type_and_default() {
        let resource = test_resource("test_res");
        let schema_str = generate_values_schema(&resource);
        let schema: Value = serde_json::from_str(&schema_str).unwrap();

        assert_eq!(schema["properties"]["replicaCount"]["type"], "integer");
        assert_eq!(schema["properties"]["replicaCount"]["default"], 1);
    }
}
