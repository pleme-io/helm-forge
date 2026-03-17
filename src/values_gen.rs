use std::collections::BTreeMap;

use iac_forge::{IacResource, IacType};

use crate::config::HelmConfig;
use crate::model::{
    AlertingConfig, ImageConfig, MonitoringConfig, PortConfig, ResourceQuantity, ResourcesConfig,
    ServiceConfig, ServicePort, ToggleConfig, ValuesYaml,
};
use crate::traits::{AttributeFilter, DefaultAttributeFilter};

/// Generate a `values.yaml` for a resource with default configuration.
#[must_use]
pub fn generate_values_yaml(resource: &IacResource) -> String {
    generate_values_yaml_with_config(resource, &HelmConfig::default())
}

/// Generate a `values.yaml` for a resource with explicit configuration.
#[must_use]
pub fn generate_values_yaml_with_config(resource: &IacResource, config: &HelmConfig) -> String {
    let values = build_values_yaml(resource, config);
    serde_yaml_ng::to_string(&values).expect("ValuesYaml serialization cannot fail")
}

/// Build a [`ValuesYaml`] struct from a resource and config.
///
/// Exposed for consumers who want to inspect or modify the struct before
/// serializing, or merge with other values.
#[must_use]
pub fn build_values_yaml(resource: &IacResource, config: &HelmConfig) -> ValuesYaml {
    let filter = DefaultAttributeFilter;
    let config_attrs = filter.config_attributes(resource);
    let secret_attrs = filter.secret_attributes(resource);

    let config_map = if config_attrs.is_empty() {
        None
    } else {
        let mut map = BTreeMap::new();
        for attr in &config_attrs {
            map.insert(
                attr.canonical_name.clone(),
                default_yaml_ng_value(&attr.iac_type),
            );
        }
        Some(map)
    };

    let secrets_map = if secret_attrs.is_empty() {
        None
    } else {
        let mut map = BTreeMap::new();
        for attr in &secret_attrs {
            map.insert(attr.canonical_name.clone(), String::new());
        }
        Some(map)
    };

    ValuesYaml {
        image: ImageConfig {
            repository: config.default_image_repository.clone(),
            tag: "latest".into(),
            pull_policy: config.image_pull_policy.clone(),
        },
        replica_count: config.replica_count,
        ports: vec![PortConfig {
            name: "http".into(),
            container_port: config.default_container_port,
            protocol: "TCP".into(),
        }],
        service: ServiceConfig {
            service_type: config.default_service_type.clone(),
            ports: vec![ServicePort {
                name: "http".into(),
                port: 80,
                target_port: "http".into(),
                protocol: "TCP".into(),
            }],
        },
        config: config_map,
        secrets: secrets_map,
        resources: ResourcesConfig {
            requests: ResourceQuantity {
                cpu: config.cpu_request.clone(),
                memory: config.memory_request.clone(),
            },
            limits: ResourceQuantity {
                cpu: config.cpu_limit.clone(),
                memory: config.memory_limit.clone(),
            },
        },
        monitoring: MonitoringConfig {
            enabled: config.monitoring_enabled,
            alerting: AlertingConfig { enabled: false },
            interval: "30s".into(),
            port: "metrics".into(),
            path: "/metrics".into(),
        },
        network_policy: ToggleConfig { enabled: config.network_policy_enabled },
        pdb: ToggleConfig { enabled: config.pdb_enabled },
        autoscaling: ToggleConfig { enabled: config.autoscaling_enabled },
    }
}

/// Map an `IacType` to a sensible `serde_yaml_ng::Value` default.
#[must_use]
pub fn default_yaml_ng_value(iac_type: &IacType) -> serde_yaml_ng::Value {
    match iac_type {
        IacType::String | IacType::Any => serde_yaml_ng::Value::String(String::new()),
        IacType::Integer => serde_yaml_ng::Value::Number(0.into()),
        IacType::Float => serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(0.0)),
        IacType::Boolean => serde_yaml_ng::Value::Bool(false),
        IacType::List(_) | IacType::Set(_) => {
            serde_yaml_ng::Value::Sequence(serde_yaml_ng::Sequence::new())
        }
        IacType::Map(_) | IacType::Object { .. } => {
            serde_yaml_ng::Value::Mapping(serde_yaml_ng::Mapping::new())
        }
        IacType::Enum { values, .. } => {
            serde_yaml_ng::Value::String(values.first().cloned().unwrap_or_default())
        }
    }
}

/// Map an `IacType` to a sensible YAML default value string (legacy API).
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
        assert!(yaml.contains("config:") || yaml.contains("config:\n"));
        assert!(yaml.contains("secrets:") || yaml.contains("secrets:\n"));
        assert!(yaml.contains("resources:"));
        assert!(yaml.contains("monitoring:"));
        assert!(yaml.contains("pdb:"));
        assert!(yaml.contains("autoscaling:"));
    }

    #[test]
    fn round_trips_through_serde() {
        let resource = test_resource("static_secret");
        let values = build_values_yaml(&resource, &HelmConfig::default());
        let yaml = serde_yaml_ng::to_string(&values).unwrap();
        let parsed: ValuesYaml = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(values, parsed);
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
        let values = build_values_yaml(&resource, &config);
        assert_eq!(values.resources.requests.cpu, "100m");
        assert_eq!(values.resources.limits.memory, "512Mi");
    }

    #[test]
    fn respects_replica_count() {
        let resource = test_resource("test");
        let values = build_values_yaml(
            &resource,
            &HelmConfig {
                replica_count: 3,
                ..HelmConfig::default()
            },
        );
        assert_eq!(values.replica_count, 3);
    }

    #[test]
    fn respects_feature_toggles() {
        let resource = test_resource("test");
        let values = build_values_yaml(
            &resource,
            &HelmConfig {
                monitoring_enabled: false,
                pdb_enabled: true,
                ..HelmConfig::default()
            },
        );
        assert!(!values.monitoring.enabled);
        assert!(values.pdb.enabled);
    }

    #[test]
    fn monitoring_has_alerting_and_fields() {
        let resource = test_resource("test");
        let values = build_values_yaml(&resource, &HelmConfig::default());
        assert!(values.monitoring.enabled);
        assert!(!values.monitoring.alerting.enabled);
        assert_eq!(values.monitoring.interval, "30s");
        assert_eq!(values.monitoring.port, "metrics");
        assert_eq!(values.monitoring.path, "/metrics");
    }

    #[test]
    fn monitoring_yaml_contains_alerting() {
        let resource = test_resource("test");
        let yaml = generate_values_yaml(&resource);
        assert!(yaml.contains("alerting:"));
        assert!(yaml.contains("interval:"));
        assert!(yaml.contains("port:"));
        assert!(yaml.contains("path:"));
    }

    #[test]
    fn empty_resource_has_no_config_or_secrets() {
        let mut resource = test_resource("empty");
        resource.attributes.clear();
        let values = build_values_yaml(&resource, &HelmConfig::default());
        assert!(values.config.is_none());
        assert!(values.secrets.is_none());
    }

    #[test]
    fn default_image_repository_is_placeholder() {
        let resource = test_resource("test");
        let values = build_values_yaml(&resource, &HelmConfig::default());
        assert_eq!(values.image.repository, "ghcr.io/pleme-io/placeholder");
    }

    #[test]
    fn custom_image_repository_propagates() {
        let resource = test_resource("test");
        let config = HelmConfig {
            default_image_repository: "ghcr.io/myorg/myapp".into(),
            ..HelmConfig::default()
        };
        let values = build_values_yaml(&resource, &config);
        assert_eq!(values.image.repository, "ghcr.io/myorg/myapp");
    }

    #[test]
    fn default_ports_section_present() {
        let resource = test_resource("test");
        let values = build_values_yaml(&resource, &HelmConfig::default());
        assert_eq!(values.ports.len(), 1);
        assert_eq!(values.ports[0].name, "http");
        assert_eq!(values.ports[0].container_port, 8080);
        assert_eq!(values.ports[0].protocol, "TCP");
    }

    #[test]
    fn default_service_section_present() {
        let resource = test_resource("test");
        let values = build_values_yaml(&resource, &HelmConfig::default());
        assert_eq!(values.service.service_type, "ClusterIP");
        assert_eq!(values.service.ports.len(), 1);
        assert_eq!(values.service.ports[0].name, "http");
        assert_eq!(values.service.ports[0].port, 80);
        assert_eq!(values.service.ports[0].target_port, "http");
        assert_eq!(values.service.ports[0].protocol, "TCP");
    }

    #[test]
    fn custom_container_port_propagates() {
        let resource = test_resource("test");
        let config = HelmConfig {
            default_container_port: 3000,
            ..HelmConfig::default()
        };
        let values = build_values_yaml(&resource, &config);
        assert_eq!(values.ports[0].container_port, 3000);
    }

    #[test]
    fn custom_service_type_propagates() {
        let resource = test_resource("test");
        let config = HelmConfig {
            default_service_type: "LoadBalancer".into(),
            ..HelmConfig::default()
        };
        let values = build_values_yaml(&resource, &config);
        assert_eq!(values.service.service_type, "LoadBalancer");
    }

    #[test]
    fn ports_yaml_contains_container_port() {
        let resource = test_resource("test");
        let yaml = generate_values_yaml(&resource);
        assert!(yaml.contains("ports:"));
        assert!(yaml.contains("containerPort: 8080"));
    }

    #[test]
    fn service_yaml_contains_type_and_ports() {
        let resource = test_resource("test");
        let yaml = generate_values_yaml(&resource);
        assert!(yaml.contains("service:"));
        assert!(yaml.contains("type: ClusterIP"));
        assert!(yaml.contains("targetPort: http"));
    }
}
