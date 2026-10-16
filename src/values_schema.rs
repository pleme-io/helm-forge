use schemars::JsonSchema;
use serde_json::{Map, Value};
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

#[must_use]
pub fn schema_value<T: JsonSchema>() -> Value {
    serde_json::to_value(schemars::schema_for!(T)).expect("a schemars schema serializes")
}

#[must_use]
pub fn strict_schema_value<T: JsonSchema>() -> Value {
    let settings = schemars::r#gen::SchemaSettings::draft07().with(|s| {
        s.option_add_null_type = false;
    });
    let schema = settings.into_generator().into_root_schema_for::<T>();
    let mut value = serde_json::to_value(schema).expect("a schemars schema serializes");
    seal(&mut value);
    bound_integers(&mut value);
    value
}

fn format_bounds(format: &str) -> Option<(i64, i64)> {
    Some(match format {
        "uint8" => (0, i64::from(u8::MAX)),
        "uint16" => (0, i64::from(u16::MAX)),
        "uint32" => (0, i64::from(u32::MAX)),
        "int8" => (i64::from(i8::MIN), i64::from(i8::MAX)),
        "int16" => (i64::from(i16::MIN), i64::from(i16::MAX)),
        "int32" => (i64::from(i32::MIN), i64::from(i32::MAX)),
        _ => return None,
    })
}

pub fn bound_integers(schema: &mut Value) {
    match schema {
        Value::Object(map) => {
            let bounds = map.get("format").and_then(Value::as_str).and_then(format_bounds);
            if let Some((min, max)) = bounds {
                map.entry("minimum").or_insert(Value::from(min));
                map.entry("maximum").or_insert(Value::from(max));
            }
            for child in map.values_mut() {
                bound_integers(child);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(bound_integers),
        _ => {}
    }
}

pub fn seal(schema: &mut Value) {
    match schema {
        Value::Object(map) => {
            if map.contains_key("properties") && !map.contains_key("additionalProperties") {
                map.insert("additionalProperties".into(), Value::Bool(false));
            }
            for child in map.values_mut() {
                seal(child);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(seal),
        _ => {}
    }
}

pub fn definition_mut<'a>(schema: &'a mut Value, name: &str) -> Option<&'a mut Map<String, Value>> {
    schema
        .get_mut("definitions")?
        .get_mut(name)?
        .as_object_mut()
}

pub fn remove_properties(object: &mut Map<String, Value>, names: &[&str]) {
    if let Some(Value::Object(props)) = object.get_mut("properties") {
        for name in names {
            props.remove(*name);
        }
    }
    if let Some(Value::Array(required)) = object.get_mut("required") {
        required.retain(|r| !names.iter().any(|n| r == n));
        if required.is_empty() {
            object.remove("required");
        }
    }
}

#[must_use]
pub fn to_json_string(schema: &Value) -> String {
    serde_json::to_string_pretty(schema).expect("a JSON value serializes")
}

#[must_use]
pub fn render(schema: &Value) -> String {
    let mut out = to_json_string(schema);
    out.push('\n');
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Write,
    Check,
}

impl Mode {
    #[must_use]
    pub fn from_args<I: IntoIterator<Item = S>, S: AsRef<str>>(args: I) -> Self {
        if args.into_iter().any(|a| a.as_ref() == "--check") {
            Self::Check
        } else {
            Self::Write
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Fresh,
    Written,
    Stale,
}

#[derive(Debug)]
pub struct StaleSchema {
    pub path: PathBuf,
    pub regenerate: String,
}

impl fmt::Display for StaleSchema {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} is stale or missing; regenerate it with `{}` and commit the result",
            self.path.display(),
            self.regenerate
        )
    }
}

impl std::error::Error for StaleSchema {}

pub fn emit(path: &Path, rendered: &str, mode: Mode) -> io::Result<Outcome> {
    let current = match std::fs::read_to_string(path) {
        Ok(s) => Some(s),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(e),
    };
    if current.as_deref() == Some(rendered) {
        return Ok(Outcome::Fresh);
    }
    match mode {
        Mode::Check => Ok(Outcome::Stale),
        Mode::Write => {
            std::fs::write(path, rendered)?;
            Ok(Outcome::Written)
        }
    }
}

pub fn check_fresh(path: &Path, rendered: &str, regenerate: &str) -> Result<(), StaleSchema> {
    match emit(path, rendered, Mode::Check) {
        Ok(Outcome::Fresh) => Ok(()),
        _ => Err(StaleSchema {
            path: path.to_path_buf(),
            regenerate: regenerate.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[allow(dead_code)]
    #[derive(Deserialize, JsonSchema)]
    #[serde(rename_all = "camelCase")]
    struct Inner {
        some_flag: bool,
    }

    #[allow(dead_code)]
    #[derive(Deserialize, JsonSchema)]
    struct Outer {
        inner: Inner,
        free: std::collections::BTreeMap<String, String>,
        keep: u8,
        drop: u8,
        maybe: Option<u32>,
    }

    #[test]
    fn seal_closes_every_owned_object_and_leaves_maps_open() {
        let v = strict_schema_value::<Outer>();
        assert_eq!(v["additionalProperties"], Value::Bool(false));
        assert_eq!(v["definitions"]["Inner"]["additionalProperties"], Value::Bool(false));
        assert_eq!(v["properties"]["free"]["additionalProperties"]["type"], "string");
        assert_eq!(v["properties"]["keep"]["maximum"], 255);
        assert_eq!(v["properties"]["maybe"]["type"], "integer");
        assert!(!v["required"].as_array().unwrap().iter().any(|r| r == "maybe"));
    }

    #[test]
    fn remove_properties_drops_property_and_requirement() {
        let mut v = schema_value::<Outer>();
        let root = v.as_object_mut().unwrap();
        remove_properties(root, &["drop"]);
        assert!(v["properties"].get("drop").is_none());
        assert!(!v["required"].as_array().unwrap().iter().any(|r| r == "drop"));
        let inner = definition_mut(&mut v, "Inner").unwrap();
        remove_properties(inner, &["someFlag"]);
        assert!(inner.get("required").is_none());
    }

    #[test]
    fn emit_writes_then_reports_fresh_and_check_reports_stale() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("values.schema.json");
        let rendered = render(&schema_value::<Inner>());
        assert!(check_fresh(&path, &rendered, "gen").is_err());
        assert_eq!(emit(&path, &rendered, Mode::Write).unwrap(), Outcome::Written);
        assert_eq!(emit(&path, &rendered, Mode::Check).unwrap(), Outcome::Fresh);
        assert!(check_fresh(&path, &rendered, "gen").is_ok());
        assert_eq!(emit(&path, "{}\n", Mode::Check).unwrap(), Outcome::Stale);
        assert!(rendered.ends_with("}\n"));
    }

    #[test]
    fn mode_reads_check_flag() {
        assert_eq!(Mode::from_args(["x", "--check"]), Mode::Check);
        assert_eq!(Mode::from_args(["x"]), Mode::Write);
    }
}
