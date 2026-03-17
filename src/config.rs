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

impl HelmConfig {
    /// Validate the configuration and return a list of error messages.
    ///
    /// Returns an empty vector when the config is valid.
    ///
    /// Checks:
    /// - `lib_chart_version` is not empty
    /// - `default_chart_version` is not empty
    /// - `cpu_request` and `memory_request` look like valid Kubernetes quantities
    /// - `replica_count` > 0
    #[must_use]
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.lib_chart_version.is_empty() {
            errors.push("lib_chart_version must not be empty".into());
        }
        if self.default_chart_version.is_empty() {
            errors.push("default_chart_version must not be empty".into());
        }
        if self.replica_count == 0 {
            errors.push("replica_count must be greater than 0".into());
        }
        if !is_k8s_quantity(&self.cpu_request) {
            errors.push(format!(
                "cpu_request '{}' does not look like a valid Kubernetes quantity",
                self.cpu_request
            ));
        }
        if !is_k8s_quantity(&self.memory_request) {
            errors.push(format!(
                "memory_request '{}' does not look like a valid Kubernetes quantity",
                self.memory_request
            ));
        }

        errors
    }
}

/// Basic check for Kubernetes resource quantities.
///
/// Accepts patterns like `100m`, `0.5`, `128Mi`, `1Gi`, `2e3`, `500`, etc.
/// This is a heuristic, not a full parser — it covers common usage.
fn is_k8s_quantity(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    // Kubernetes quantities: digits (optionally with decimal/exponent), followed by
    // an optional suffix: m | k | M | G | T | P | E | Ki | Mi | Gi | Ti | Pi | Ei
    let suffixes = [
        "Ki", "Mi", "Gi", "Ti", "Pi", "Ei", // binary
        "m", "k", "M", "G", "T", "P", "E",  // decimal (m = milli)
    ];

    for suffix in &suffixes {
        if let Some(prefix) = s.strip_suffix(suffix) {
            return !prefix.is_empty() && prefix.bytes().all(|b| b.is_ascii_digit() || b == b'.');
        }
    }

    // Plain number (no suffix): e.g., "0.5", "100", "2e3"
    s.bytes()
        .all(|b| b.is_ascii_digit() || b == b'.' || b == b'e' || b == b'E')
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

    // ── Validation tests ─────────────────────────────────────────────────

    #[test]
    fn default_config_is_valid() {
        let cfg = HelmConfig::default();
        assert!(cfg.validate().is_empty());
    }

    #[test]
    fn validate_empty_lib_chart_version() {
        let cfg = HelmConfig {
            lib_chart_version: String::new(),
            ..HelmConfig::default()
        };
        let errors = cfg.validate();
        assert!(errors.iter().any(|e| e.contains("lib_chart_version")));
    }

    #[test]
    fn validate_empty_default_chart_version() {
        let cfg = HelmConfig {
            default_chart_version: String::new(),
            ..HelmConfig::default()
        };
        let errors = cfg.validate();
        assert!(errors.iter().any(|e| e.contains("default_chart_version")));
    }

    #[test]
    fn validate_zero_replica_count() {
        let cfg = HelmConfig {
            replica_count: 0,
            ..HelmConfig::default()
        };
        let errors = cfg.validate();
        assert!(errors.iter().any(|e| e.contains("replica_count")));
    }

    #[test]
    fn validate_invalid_cpu_request() {
        let cfg = HelmConfig {
            cpu_request: "not-a-quantity".into(),
            ..HelmConfig::default()
        };
        let errors = cfg.validate();
        assert!(errors.iter().any(|e| e.contains("cpu_request")));
    }

    #[test]
    fn validate_invalid_memory_request() {
        let cfg = HelmConfig {
            memory_request: "".into(),
            ..HelmConfig::default()
        };
        let errors = cfg.validate();
        assert!(errors.iter().any(|e| e.contains("memory_request")));
    }

    #[test]
    fn validate_valid_quantities() {
        for q in &["50m", "100m", "0.5", "1", "128Mi", "1Gi", "256Ki"] {
            assert!(is_k8s_quantity(q), "{q} should be a valid quantity");
        }
    }

    #[test]
    fn validate_invalid_quantities() {
        for q in &["", "abc", "Mi", "-5m", "not-valid"] {
            assert!(!is_k8s_quantity(q), "{q} should not be a valid quantity");
        }
    }

    #[test]
    fn validate_multiple_errors_at_once() {
        let cfg = HelmConfig {
            lib_chart_version: String::new(),
            default_chart_version: String::new(),
            replica_count: 0,
            cpu_request: "bad".into(),
            memory_request: "bad".into(),
            ..HelmConfig::default()
        };
        let errors = cfg.validate();
        assert_eq!(errors.len(), 5);
    }
}
