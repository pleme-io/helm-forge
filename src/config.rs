/// Configuration for the Helm chart generator.
///
/// All fields have sensible defaults matching pleme-io conventions.
/// Override individual fields to customise generated chart metadata,
/// library chart dependency, default Kubernetes resource limits,
/// replica count, and feature toggles.
#[derive(Debug, Clone)]
pub struct HelmConfig {
    // ── Chart metadata ──────────────────────────────────────────────────
    /// Name of the library chart dependency (default: `"pleme-lib"`).
    pub lib_chart_name: String,
    /// Semver constraint for the library chart (default: `"~0.4.0"`).
    pub lib_chart_version: String,
    /// Helm repository URL for the library chart (default: `"file://../pleme-lib"`).
    pub lib_chart_repository: String,
    /// Default chart version for generated charts (default: `"0.1.0"`).
    pub default_chart_version: String,
    /// Default appVersion for generated charts (default: `"1.0.0"`).
    pub default_app_version: String,

    // ── Container defaults ──────────────────────────────────────────────
    /// Default replica count (default: `1`).
    pub replica_count: u32,
    /// Default image pull policy (default: `"Always"`).
    pub image_pull_policy: String,
    /// Default CPU request (default: `"50m"`).
    pub cpu_request: String,
    /// Default memory request (default: `"64Mi"`).
    pub memory_request: String,
    /// Default CPU limit (default: `"200m"`).
    pub cpu_limit: String,
    /// Default memory limit (default: `"256Mi"`).
    pub memory_limit: String,

    // ── Feature toggles ─────────────────────────────────────────────────
    /// Enable monitoring (ServiceMonitor) by default (default: `true`).
    pub monitoring_enabled: bool,
    /// Enable network policy by default (default: `true`).
    pub network_policy_enabled: bool,
    /// Enable PodDisruptionBudget by default (default: `false`).
    pub pdb_enabled: bool,
    /// Enable HorizontalPodAutoscaler by default (default: `false`).
    pub autoscaling_enabled: bool,
}

impl Default for HelmConfig {
    fn default() -> Self {
        Self {
            lib_chart_name: String::from("pleme-lib"),
            lib_chart_version: String::from("~0.4.0"),
            lib_chart_repository: String::from("file://../pleme-lib"),
            default_chart_version: String::from("0.1.0"),
            default_app_version: String::from("1.0.0"),
            replica_count: 1,
            image_pull_policy: String::from("Always"),
            cpu_request: String::from("50m"),
            memory_request: String::from("64Mi"),
            cpu_limit: String::from("200m"),
            memory_limit: String::from("256Mi"),
            monitoring_enabled: true,
            network_policy_enabled: true,
            pdb_enabled: false,
            autoscaling_enabled: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_pleme_lib() {
        let cfg = HelmConfig::default();
        assert_eq!(cfg.lib_chart_name, "pleme-lib");
        assert_eq!(cfg.lib_chart_version, "~0.4.0");
    }

    #[test]
    fn default_config_reasonable_defaults() {
        let cfg = HelmConfig::default();
        assert_eq!(cfg.replica_count, 1);
        assert_eq!(cfg.image_pull_policy, "Always");
        assert!(cfg.monitoring_enabled);
        assert!(cfg.network_policy_enabled);
        assert!(!cfg.pdb_enabled);
        assert!(!cfg.autoscaling_enabled);
    }

    #[test]
    fn config_is_clonable() {
        let cfg = HelmConfig::default();
        let cfg2 = cfg.clone();
        assert_eq!(cfg.lib_chart_name, cfg2.lib_chart_name);
        assert_eq!(cfg.replica_count, cfg2.replica_count);
    }
}
