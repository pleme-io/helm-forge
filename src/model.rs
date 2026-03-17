//! Central data structures for Helm chart artifacts.
//!
//! These structs model the YAML files that Helm charts contain. They are
//! serializable via `serde` for type-safe YAML generation, and deserializable
//! for round-trip testing.
//!
//! # Architecture
//!
//! - **Pure YAML** files (Chart.yaml, values.yaml) use `#[derive(Serialize, Deserialize)]`
//!   structs serialized via `serde_yaml_ng`.
//! - **Helm template** files use the [`HelmNode`] AST (see [`helm_ast`](crate::helm_ast)),
//!   which models Go template syntax as a typed enum.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

// ── Chart.yaml ──────────────────────────────────────────────────────────────

/// A Helm `Chart.yaml` file.
///
/// Serializes to valid Helm Chart.yaml via `serde_yaml_ng::to_string()`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChartYaml {
    pub api_version: String,
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub chart_type: String,
    pub version: String,
    pub app_version: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<ChartDependency>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub annotations: BTreeMap<String, String>,
}

/// A dependency entry in Chart.yaml.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChartDependency {
    pub name: String,
    pub version: String,
    pub repository: String,
}

// ── values.yaml ─────────────────────────────────────────────────────────────

/// A Helm `values.yaml` file.
///
/// Serializes to valid Helm values.yaml via `serde_yaml_ng::to_string()`.
/// Supports optional sections that are omitted when `None`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ValuesYaml {
    pub image: ImageConfig,
    pub replica_count: u32,
    pub ports: Vec<PortConfig>,
    pub service: ServiceConfig,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<BTreeMap<String, serde_yaml_ng::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secrets: Option<BTreeMap<String, String>>,
    pub resources: ResourcesConfig,
    pub monitoring: MonitoringConfig,
    pub network_policy: ToggleConfig,
    pub pdb: ToggleConfig,
    pub autoscaling: ToggleConfig,
}

/// Container port configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PortConfig {
    pub name: String,
    pub container_port: u16,
    pub protocol: String,
}

/// Kubernetes Service configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServiceConfig {
    #[serde(rename = "type")]
    pub service_type: String,
    pub ports: Vec<ServicePort>,
}

/// A port entry in the Service spec.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ServicePort {
    pub name: String,
    pub port: u16,
    pub target_port: String,
    pub protocol: String,
}

/// Container image configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImageConfig {
    pub repository: String,
    pub tag: String,
    pub pull_policy: String,
}

/// Kubernetes resource requests/limits.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourcesConfig {
    pub requests: ResourceQuantity,
    pub limits: ResourceQuantity,
}

/// CPU and memory quantities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceQuantity {
    pub cpu: String,
    pub memory: String,
}

/// Monitoring configuration with alerting sub-section.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MonitoringConfig {
    pub enabled: bool,
    pub alerting: AlertingConfig,
    pub interval: String,
    pub port: String,
    pub path: String,
}

/// Alerting sub-configuration for PrometheusRule generation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AlertingConfig {
    pub enabled: bool,
}

/// Simple `{ enabled: bool }` toggle for features.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToggleConfig {
    pub enabled: bool,
}

impl ToggleConfig {
    #[must_use]
    pub fn on() -> Self {
        Self { enabled: true }
    }

    #[must_use]
    pub fn off() -> Self {
        Self { enabled: false }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chart_yaml_round_trips() {
        let chart = ChartYaml {
            api_version: "v2".into(),
            name: "test-chart".into(),
            description: "A test chart".into(),
            chart_type: "application".into(),
            version: "0.1.0".into(),
            app_version: "1.0.0".into(),
            dependencies: vec![ChartDependency {
                name: "pleme-lib".into(),
                version: "~0.4.0".into(),
                repository: "file://../pleme-lib".into(),
            }],
            annotations: BTreeMap::from([
                ("pleme.io/generated".into(), "true".into()),
            ]),
        };

        let yaml = serde_yaml_ng::to_string(&chart).unwrap();
        let parsed: ChartYaml = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(chart, parsed);
    }

    #[test]
    fn chart_yaml_produces_correct_field_names() {
        let chart = ChartYaml {
            api_version: "v2".into(),
            name: "test".into(),
            description: "desc".into(),
            chart_type: "application".into(),
            version: "0.1.0".into(),
            app_version: "1.0.0".into(),
            dependencies: vec![],
            annotations: BTreeMap::new(),
        };
        let yaml = serde_yaml_ng::to_string(&chart).unwrap();
        assert!(yaml.contains("apiVersion: v2"));
        assert!(yaml.contains("appVersion:"));
        assert!(yaml.contains("type: application"));
        // No empty sections
        assert!(!yaml.contains("dependencies"));
        assert!(!yaml.contains("annotations"));
    }

    fn default_monitoring() -> MonitoringConfig {
        MonitoringConfig {
            enabled: true,
            alerting: AlertingConfig { enabled: false },
            interval: "30s".into(),
            port: "metrics".into(),
            path: "/metrics".into(),
        }
    }

    fn default_ports() -> Vec<PortConfig> {
        vec![PortConfig {
            name: "http".into(),
            container_port: 8080,
            protocol: "TCP".into(),
        }]
    }

    fn default_service() -> ServiceConfig {
        ServiceConfig {
            service_type: "ClusterIP".into(),
            ports: vec![ServicePort {
                name: "http".into(),
                port: 80,
                target_port: "http".into(),
                protocol: "TCP".into(),
            }],
        }
    }

    #[test]
    fn values_yaml_round_trips() {
        let values = ValuesYaml {
            image: ImageConfig {
                repository: "nginx".into(),
                tag: "latest".into(),
                pull_policy: "Always".into(),
            },
            replica_count: 2,
            ports: default_ports(),
            service: default_service(),
            config: Some(BTreeMap::from([
                ("key".into(), serde_yaml_ng::Value::String("value".into())),
            ])),
            secrets: Some(BTreeMap::from([("api_key".into(), "".into())])),
            resources: ResourcesConfig {
                requests: ResourceQuantity {
                    cpu: "50m".into(),
                    memory: "64Mi".into(),
                },
                limits: ResourceQuantity {
                    cpu: "200m".into(),
                    memory: "256Mi".into(),
                },
            },
            monitoring: default_monitoring(),
            network_policy: ToggleConfig::on(),
            pdb: ToggleConfig::off(),
            autoscaling: ToggleConfig::off(),
        };

        let yaml = serde_yaml_ng::to_string(&values).unwrap();
        let parsed: ValuesYaml = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(values, parsed);
    }

    #[test]
    fn values_yaml_omits_none_sections() {
        let values = ValuesYaml {
            image: ImageConfig {
                repository: "ghcr.io/pleme-io/placeholder".into(),
                tag: "latest".into(),
                pull_policy: "Always".into(),
            },
            replica_count: 1,
            ports: default_ports(),
            service: default_service(),
            config: None,
            secrets: None,
            resources: ResourcesConfig {
                requests: ResourceQuantity { cpu: "50m".into(), memory: "64Mi".into() },
                limits: ResourceQuantity { cpu: "200m".into(), memory: "256Mi".into() },
            },
            monitoring: default_monitoring(),
            network_policy: ToggleConfig::on(),
            pdb: ToggleConfig::off(),
            autoscaling: ToggleConfig::off(),
        };
        let yaml = serde_yaml_ng::to_string(&values).unwrap();
        assert!(!yaml.contains("config:"));
        assert!(!yaml.contains("secrets:"));
    }

    #[test]
    fn toggle_config_convenience() {
        assert!(ToggleConfig::on().enabled);
        assert!(!ToggleConfig::off().enabled);
    }
}
