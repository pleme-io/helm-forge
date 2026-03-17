use iac_forge::IacType;
use serde_json::{Map, Value};

/// Convert an `IacType` to a JSON Schema object.
#[must_use]
pub fn iac_type_to_json_schema(iac_type: &IacType) -> Value {
    match iac_type {
        IacType::String => json_schema_string(),
        IacType::Integer => json_schema_type("integer"),
        IacType::Float => json_schema_type("number"),
        IacType::Boolean => json_schema_type("boolean"),
        IacType::List(inner) | IacType::Set(inner) => {
            let mut obj = Map::new();
            obj.insert("type".into(), Value::String("array".into()));
            obj.insert("items".into(), iac_type_to_json_schema(inner));
            if matches!(iac_type, IacType::Set(_)) {
                obj.insert("uniqueItems".into(), Value::Bool(true));
            }
            Value::Object(obj)
        }
        IacType::Map(inner) => {
            let mut obj = Map::new();
            obj.insert("type".into(), Value::String("object".into()));
            obj.insert(
                "additionalProperties".into(),
                iac_type_to_json_schema(inner),
            );
            Value::Object(obj)
        }
        IacType::Object { fields, .. } => {
            let mut props = Map::new();
            let mut required = Vec::new();

            for field in fields {
                props.insert(
                    field.canonical_name.clone(),
                    iac_type_to_json_schema(&field.iac_type),
                );
                if field.required {
                    required.push(Value::String(field.canonical_name.clone()));
                }
            }

            let mut obj = Map::new();
            obj.insert("type".into(), Value::String("object".into()));
            obj.insert("properties".into(), Value::Object(props));
            if !required.is_empty() {
                obj.insert("required".into(), Value::Array(required));
            }
            Value::Object(obj)
        }
        IacType::Enum { values, .. } => {
            let mut obj = Map::new();
            obj.insert("type".into(), Value::String("string".into()));
            obj.insert(
                "enum".into(),
                Value::Array(values.iter().map(|v| Value::String(v.clone())).collect()),
            );
            Value::Object(obj)
        }
        IacType::Any => {
            // Empty schema allows any type
            Value::Object(Map::new())
        }
    }
}

fn json_schema_string() -> Value {
    json_schema_type("string")
}

fn json_schema_type(t: &str) -> Value {
    let mut obj = Map::new();
    obj.insert("type".into(), Value::String(t.into()));
    Value::Object(obj)
}

#[cfg(test)]
mod tests {
    use super::*;
    use iac_forge::IacType;

    #[test]
    fn string_maps_to_string() {
        let schema = iac_type_to_json_schema(&IacType::String);
        assert_eq!(schema["type"], "string");
    }

    #[test]
    fn integer_maps_to_integer() {
        let schema = iac_type_to_json_schema(&IacType::Integer);
        assert_eq!(schema["type"], "integer");
    }

    #[test]
    fn boolean_maps_to_boolean() {
        let schema = iac_type_to_json_schema(&IacType::Boolean);
        assert_eq!(schema["type"], "boolean");
    }

    #[test]
    fn float_maps_to_number() {
        let schema = iac_type_to_json_schema(&IacType::Float);
        assert_eq!(schema["type"], "number");
    }

    #[test]
    fn list_maps_to_array() {
        let schema = iac_type_to_json_schema(&IacType::List(Box::new(IacType::String)));
        assert_eq!(schema["type"], "array");
        assert_eq!(schema["items"]["type"], "string");
        assert!(schema.get("uniqueItems").is_none());
    }

    #[test]
    fn set_maps_to_unique_array() {
        let schema = iac_type_to_json_schema(&IacType::Set(Box::new(IacType::Integer)));
        assert_eq!(schema["type"], "array");
        assert_eq!(schema["items"]["type"], "integer");
        assert_eq!(schema["uniqueItems"], true);
    }

    #[test]
    fn map_maps_to_object_with_additional_properties() {
        let schema = iac_type_to_json_schema(&IacType::Map(Box::new(IacType::String)));
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["additionalProperties"]["type"], "string");
    }

    #[test]
    fn enum_maps_to_string_with_enum() {
        let schema = iac_type_to_json_schema(&IacType::Enum {
            values: vec!["a".into(), "b".into()],
            underlying: Box::new(IacType::String),
        });
        assert_eq!(schema["type"], "string");
        assert_eq!(schema["enum"], serde_json::json!(["a", "b"]));
    }

    #[test]
    fn any_maps_to_empty_schema() {
        let schema = iac_type_to_json_schema(&IacType::Any);
        assert_eq!(schema, Value::Object(Map::new()));
    }
}
