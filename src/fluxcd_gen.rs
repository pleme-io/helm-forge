//! FluxCD HelmRelease manifest generator.
//!
//! Generates FluxCD v2 `HelmRelease` CRDs and a `kustomization.yaml`
//! for deploying Helm charts via GitOps.

use iac_forge::ir::IacResource;
use iac_forge::to_kebab_case;

use crate::naming::validate_dns1123;

// ── Configuration ────────────────────────────────────────────────────────────

/// Configuration for FluxCD HelmRelease generation.
#[derive(Debug, Clone)]
pub struct FluxCdConfig {
    /// Namespace for the HelmRelease (default: `"akeyless-system"`).
    pub namespace: String,
    /// Name of the GitRepository/HelmRepository source (default: `"helm-akeyless-gen"`).
    pub source_name: String,
    /// Namespace of the source (default: `"flux-system"`).
    pub source_namespace: String,
    /// Kind of the source reference (default: `"GitRepository"`).
    pub source_kind: String,
    /// Reconciliation interval (default: `"5m"`).
    pub interval: String,
    /// Chart source polling interval (default: `"1h"`).
    pub chart_interval: String,
    /// Number of install/upgrade remediation retries (default: `3`).
    pub retries: u32,
}

impl Default for FluxCdConfig {
    fn default() -> Self {
        Self {
            namespace: String::from("akeyless-system"),
            source_name: String::from("helm-akeyless-gen"),
            source_namespace: String::from("flux-system"),
            source_kind: String::from("GitRepository"),
            interval: String::from("5m"),
            chart_interval: String::from("1h"),
            retries: 3,
        }
    }
}

// ── Trait ─────────────────────────────────────────────────────────────────────

/// Generator for FluxCD HelmRelease manifests.
pub trait FluxCdGenerator: std::fmt::Debug + Send + Sync {
    /// Generate a FluxCD `HelmRelease` YAML manifest for the given resource.
    fn generate(&self, resource: &IacResource, provider_name: &str) -> String;
}

// ── Default implementation ───────────────────────────────────────────────────

/// Default `FluxCdGenerator` backed by [`FluxCdConfig`].
#[derive(Debug, Clone)]
pub struct DefaultFluxCdGenerator {
    /// The configuration used for generation.
    pub config: FluxCdConfig,
}

impl FluxCdGenerator for DefaultFluxCdGenerator {
    fn generate(&self, resource: &IacResource, provider_name: &str) -> String {
        generate_helmrelease(resource, provider_name, &self.config)
    }
}

// ── Standalone generator functions ───────────────────────────────────────────

/// Generate a FluxCD `HelmRelease` YAML manifest for a single resource.
///
/// The chart name is derived from the resource name using the Helm naming
/// convention (kebab-case, DNS-1123 validated).
#[must_use]
pub fn generate_helmrelease(
    resource: &IacResource,
    _provider_name: &str,
    config: &FluxCdConfig,
) -> String {
    let chart_name = validate_dns1123(&resource.name)
        .map_or_else(|_| to_kebab_case(&resource.name), |r| r.name().to_string());

    format!(
        "\
apiVersion: helm.toolkit.fluxcd.io/v2
kind: HelmRelease
metadata:
  name: {chart_name}
  namespace: {namespace}
spec:
  interval: {interval}
  chart:
    spec:
      chart: charts/{chart_name}
      sourceRef:
        kind: {source_kind}
        name: {source_name}
        namespace: {source_namespace}
      interval: {chart_interval}
  install:
    remediation:
      retries: {retries}
  upgrade:
    remediation:
      retries: {retries}
  values: {{}}
",
        namespace = config.namespace,
        interval = config.interval,
        source_kind = config.source_kind,
        source_name = config.source_name,
        source_namespace = config.source_namespace,
        chart_interval = config.chart_interval,
        retries = config.retries,
    )
}

/// Generate a Kustomize `kustomization.yaml` listing all HelmRelease files.
///
/// The resource list is sorted alphabetically for deterministic output.
#[must_use]
pub fn generate_kustomization(resources: &[IacResource]) -> String {
    let mut filenames: Vec<String> = resources
        .iter()
        .map(|r| {
            let chart_name = validate_dns1123(&r.name)
                .map_or_else(|_| to_kebab_case(&r.name), |result| result.name().to_string());
            format!("helmrelease-{chart_name}.yaml")
        })
        .collect();
    filenames.sort();

    let mut output = String::from(
        "\
apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization
resources:\n",
    );

    for filename in &filenames {
        output.push_str(&format!("  - {filename}\n"));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use iac_forge::testing::{test_resource, test_resource_with_type};
    use iac_forge::IacType;

    // ── FluxCdConfig defaults ────────────────────────────────────────────

    #[test]
    fn default_config_has_sensible_values() {
        let cfg = FluxCdConfig::default();
        assert_eq!(cfg.namespace, "akeyless-system");
        assert_eq!(cfg.source_name, "helm-akeyless-gen");
        assert_eq!(cfg.source_namespace, "flux-system");
        assert_eq!(cfg.source_kind, "GitRepository");
        assert_eq!(cfg.interval, "5m");
        assert_eq!(cfg.chart_interval, "1h");
        assert_eq!(cfg.retries, 3);
    }

    #[test]
    fn config_is_clonable() {
        let cfg = FluxCdConfig::default();
        let cfg2 = cfg.clone();
        assert_eq!(cfg.namespace, cfg2.namespace);
        assert_eq!(cfg.retries, cfg2.retries);
    }

    // ── HelmRelease YAML structure ───────────────────────────────────────

    #[test]
    fn helmrelease_has_correct_api_version_and_kind() {
        let resource = test_resource("static_secret");
        let yaml = generate_helmrelease(&resource, "akeyless", &FluxCdConfig::default());
        assert!(yaml.contains("apiVersion: helm.toolkit.fluxcd.io/v2"));
        assert!(yaml.contains("kind: HelmRelease"));
    }

    #[test]
    fn helmrelease_uses_chart_name_from_resource() {
        let resource = test_resource("auth_method_api_key");
        let yaml = generate_helmrelease(&resource, "akeyless", &FluxCdConfig::default());
        assert!(yaml.contains("name: auth-method-api-key"));
        assert!(yaml.contains("chart: charts/auth-method-api-key"));
    }

    #[test]
    fn helmrelease_uses_default_namespace() {
        let resource = test_resource("test");
        let yaml = generate_helmrelease(&resource, "akeyless", &FluxCdConfig::default());
        assert!(yaml.contains("namespace: akeyless-system"));
    }

    #[test]
    fn helmrelease_uses_default_source_ref() {
        let resource = test_resource("test");
        let yaml = generate_helmrelease(&resource, "akeyless", &FluxCdConfig::default());
        assert!(yaml.contains("kind: GitRepository"));
        assert!(yaml.contains("name: helm-akeyless-gen"));
        assert!(yaml.contains("namespace: flux-system"));
    }

    #[test]
    fn helmrelease_uses_default_intervals() {
        let resource = test_resource("test");
        let yaml = generate_helmrelease(&resource, "akeyless", &FluxCdConfig::default());
        assert!(yaml.contains("interval: 5m"));
        assert!(yaml.contains("interval: 1h"));
    }

    #[test]
    fn helmrelease_uses_default_retries() {
        let resource = test_resource("test");
        let yaml = generate_helmrelease(&resource, "akeyless", &FluxCdConfig::default());
        assert!(yaml.contains("retries: 3"));
    }

    #[test]
    fn helmrelease_has_empty_values() {
        let resource = test_resource("test");
        let yaml = generate_helmrelease(&resource, "akeyless", &FluxCdConfig::default());
        assert!(yaml.contains("values: {}"));
    }

    #[test]
    fn helmrelease_has_install_and_upgrade_remediation() {
        let resource = test_resource("test");
        let yaml = generate_helmrelease(&resource, "akeyless", &FluxCdConfig::default());
        assert!(yaml.contains("install:"));
        assert!(yaml.contains("upgrade:"));
        assert!(yaml.contains("remediation:"));
    }

    // ── Custom config is respected ───────────────────────────────────────

    #[test]
    fn helmrelease_respects_custom_namespace() {
        let config = FluxCdConfig {
            namespace: "custom-ns".into(),
            ..FluxCdConfig::default()
        };
        let resource = test_resource("test");
        let yaml = generate_helmrelease(&resource, "akeyless", &config);
        assert!(yaml.contains("namespace: custom-ns"));
    }

    #[test]
    fn helmrelease_respects_custom_source() {
        let config = FluxCdConfig {
            source_name: "my-repo".into(),
            source_namespace: "my-ns".into(),
            source_kind: "HelmRepository".into(),
            ..FluxCdConfig::default()
        };
        let resource = test_resource("test");
        let yaml = generate_helmrelease(&resource, "akeyless", &config);
        assert!(yaml.contains("kind: HelmRepository"));
        assert!(yaml.contains("name: my-repo"));
        assert!(yaml.contains("namespace: my-ns"));
    }

    #[test]
    fn helmrelease_respects_custom_intervals() {
        let config = FluxCdConfig {
            interval: "10m".into(),
            chart_interval: "2h".into(),
            ..FluxCdConfig::default()
        };
        let resource = test_resource("test");
        let yaml = generate_helmrelease(&resource, "akeyless", &config);
        assert!(yaml.contains("interval: 10m"));
        assert!(yaml.contains("interval: 2h"));
    }

    #[test]
    fn helmrelease_respects_custom_retries() {
        let config = FluxCdConfig {
            retries: 5,
            ..FluxCdConfig::default()
        };
        let resource = test_resource("test");
        let yaml = generate_helmrelease(&resource, "akeyless", &config);
        assert!(yaml.contains("retries: 5"));
    }

    // ── HelmRelease YAML is parseable ────────────────────────────────────

    #[test]
    fn helmrelease_is_valid_yaml() {
        let resource = test_resource("static_secret");
        let yaml_str = generate_helmrelease(&resource, "akeyless", &FluxCdConfig::default());
        let doc: serde_json::Value =
            serde_yaml_ng::from_str(&yaml_str).expect("HelmRelease must be valid YAML");
        assert_eq!(doc["apiVersion"], "helm.toolkit.fluxcd.io/v2");
        assert_eq!(doc["kind"], "HelmRelease");
        assert_eq!(doc["metadata"]["name"], "static-secret");
        assert_eq!(doc["metadata"]["namespace"], "akeyless-system");
        assert_eq!(doc["spec"]["chart"]["spec"]["chart"], "charts/static-secret");
    }

    // ── DefaultFluxCdGenerator trait impl ────────────────────────────────

    #[test]
    fn default_generator_produces_valid_output() {
        let generator = DefaultFluxCdGenerator {
            config: FluxCdConfig::default(),
        };
        let resource = test_resource("test_res");
        let output = generator.generate(&resource, "akeyless");
        assert!(output.contains("apiVersion: helm.toolkit.fluxcd.io/v2"));
        assert!(output.contains("name: test-res"));
    }

    // ── Kustomization generation ─────────────────────────────────────────

    #[test]
    fn kustomization_lists_all_resources() {
        let resources = vec![
            test_resource("auth_method_api_key"),
            test_resource("static_secret"),
            test_resource("target_ssh"),
        ];
        let kustomization = generate_kustomization(&resources);
        assert!(kustomization.contains("apiVersion: kustomize.config.k8s.io/v1beta1"));
        assert!(kustomization.contains("kind: Kustomization"));
        assert!(kustomization.contains("helmrelease-auth-method-api-key.yaml"));
        assert!(kustomization.contains("helmrelease-static-secret.yaml"));
        assert!(kustomization.contains("helmrelease-target-ssh.yaml"));
    }

    #[test]
    fn kustomization_is_sorted_alphabetically() {
        let resources = vec![
            test_resource("zzz_last"),
            test_resource("aaa_first"),
            test_resource("mmm_middle"),
        ];
        let kustomization = generate_kustomization(&resources);
        let lines: Vec<&str> = kustomization.lines().collect();
        let resource_lines: Vec<&&str> = lines.iter().filter(|l| l.starts_with("  - ")).collect();
        assert_eq!(resource_lines.len(), 3);
        assert!(resource_lines[0].contains("aaa-first"));
        assert!(resource_lines[1].contains("mmm-middle"));
        assert!(resource_lines[2].contains("zzz-last"));
    }

    #[test]
    fn kustomization_empty_resources() {
        let kustomization = generate_kustomization(&[]);
        assert!(kustomization.contains("apiVersion: kustomize.config.k8s.io/v1beta1"));
        assert!(kustomization.contains("kind: Kustomization"));
        assert!(kustomization.contains("resources:"));
        // No resource entries
        assert!(!kustomization.contains("  - "));
    }

    #[test]
    fn kustomization_is_valid_yaml() {
        let resources = vec![
            test_resource("auth_method_api_key"),
            test_resource("static_secret"),
        ];
        let yaml_str = generate_kustomization(&resources);
        let doc: serde_json::Value =
            serde_yaml_ng::from_str(&yaml_str).expect("kustomization must be valid YAML");
        assert_eq!(doc["apiVersion"], "kustomize.config.k8s.io/v1beta1");
        assert_eq!(doc["kind"], "Kustomization");
        let res_arr = doc["resources"].as_array().expect("resources should be array");
        assert_eq!(res_arr.len(), 2);
    }

    // ── Trait is object-safe ─────────────────────────────────────────────

    #[test]
    fn fluxcd_generator_trait_is_object_safe() {
        let _: Box<dyn FluxCdGenerator> = Box::new(DefaultFluxCdGenerator {
            config: FluxCdConfig::default(),
        });
    }

    // ── Edge cases ───────────────────────────────────────────────────────

    #[test]
    fn helmrelease_with_snake_case_resource_name() {
        let resource = test_resource_with_type("auth_method_aws_iam", "field", IacType::String);
        let yaml = generate_helmrelease(&resource, "akeyless", &FluxCdConfig::default());
        assert!(yaml.contains("name: auth-method-aws-iam"));
        assert!(yaml.contains("chart: charts/auth-method-aws-iam"));
    }
}
