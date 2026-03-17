use iac_forge::backend::{ArtifactKind, NamingConvention};
use iac_forge::to_kebab_case;

/// Maximum length of a DNS-1123 label (RFC 1123).
const DNS_1123_MAX_LEN: usize = 63;

/// Result of DNS-1123 validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dns1123Result {
    /// The name is valid as-is.
    Valid(String),
    /// The name was truncated to fit the 63-character limit.
    Truncated(String),
}

impl Dns1123Result {
    /// Return the validated (possibly truncated) name.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Valid(n) | Self::Truncated(n) => n,
        }
    }
}

/// Validate and normalise a resource name to conform to DNS-1123 label rules.
///
/// 1. Converts to kebab-case (lowercase + hyphens).
/// 2. Strips any characters that are not `[a-z0-9-]`.
/// 3. Strips leading/trailing hyphens.
/// 4. Truncates to 63 characters (trimming trailing hyphens after truncation).
///
/// Returns `Err` if the resulting name is empty or contains only invalid characters.
pub fn validate_dns1123(raw: &str) -> Result<Dns1123Result, String> {
    let kebab = to_kebab_case(raw).to_ascii_lowercase();

    // Keep only lowercase alphanumeric and hyphens.
    let cleaned: String = kebab
        .chars()
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
        .collect();

    // Strip leading/trailing hyphens.
    let trimmed = cleaned.trim_matches('-');

    if trimmed.is_empty() {
        return Err(format!(
            "resource name '{raw}' produces an empty DNS-1123 label after normalisation"
        ));
    }

    if trimmed.len() <= DNS_1123_MAX_LEN {
        return Ok(Dns1123Result::Valid(trimmed.to_string()));
    }

    // Truncate and strip any trailing hyphen introduced by the cut.
    let truncated = trimmed[..DNS_1123_MAX_LEN].trim_end_matches('-');
    if truncated.is_empty() {
        return Err(format!(
            "resource name '{raw}' is too long and truncation yields an empty label"
        ));
    }

    Ok(Dns1123Result::Truncated(truncated.to_string()))
}

/// Helm chart naming convention.
///
/// Resource names are kebab-cased for chart directory names.
/// Field names are preserved in their canonical (snake_case) form for values.yaml.
#[derive(Debug, Clone, Copy)]
pub struct HelmNaming;

impl NamingConvention for HelmNaming {
    fn resource_type_name(&self, resource_name: &str, _provider_name: &str) -> String {
        to_kebab_case(resource_name)
    }

    fn file_name(&self, resource_name: &str, kind: &ArtifactKind) -> String {
        let base = to_kebab_case(resource_name);
        match kind {
            ArtifactKind::Resource => format!("charts/{base}/Chart.yaml"),
            ArtifactKind::Schema => format!("charts/{base}/values.schema.json"),
            ArtifactKind::Test => format!("charts/{base}/tests/deployment_test.yaml"),
            _ => format!("charts/{base}/{base}.yaml"),
        }
    }

    fn field_name(&self, api_name: &str) -> String {
        iac_forge::to_snake_case(api_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_type_name_kebab_cases() {
        let naming = HelmNaming;
        assert_eq!(
            naming.resource_type_name("static_secret", "akeyless"),
            "static-secret"
        );
    }

    #[test]
    fn field_name_snake_cases() {
        let naming = HelmNaming;
        assert_eq!(
            naming.field_name("bound-aws-account-id"),
            "bound_aws_account_id"
        );
    }

    #[test]
    fn file_name_resource() {
        let naming = HelmNaming;
        assert_eq!(
            naming.file_name("static_secret", &ArtifactKind::Resource),
            "charts/static-secret/Chart.yaml"
        );
    }

    #[test]
    fn file_name_schema() {
        let naming = HelmNaming;
        assert_eq!(
            naming.file_name("static_secret", &ArtifactKind::Schema),
            "charts/static-secret/values.schema.json"
        );
    }

    #[test]
    fn file_name_test() {
        let naming = HelmNaming;
        assert_eq!(
            naming.file_name("static_secret", &ArtifactKind::Test),
            "charts/static-secret/tests/deployment_test.yaml"
        );
    }

    // ── DNS-1123 validation tests ────────────────────────────────────────

    #[test]
    fn dns1123_valid_simple_name() {
        let result = validate_dns1123("my-app").unwrap();
        assert_eq!(result, Dns1123Result::Valid("my-app".into()));
        assert_eq!(result.name(), "my-app");
    }

    #[test]
    fn dns1123_valid_snake_case_converted() {
        let result = validate_dns1123("static_secret").unwrap();
        assert_eq!(result, Dns1123Result::Valid("static-secret".into()));
    }

    #[test]
    fn dns1123_valid_single_char() {
        let result = validate_dns1123("a").unwrap();
        assert_eq!(result, Dns1123Result::Valid("a".into()));
    }

    #[test]
    fn dns1123_valid_numeric() {
        let result = validate_dns1123("app123").unwrap();
        assert_eq!(result, Dns1123Result::Valid("app123".into()));
    }

    #[test]
    fn dns1123_valid_max_length() {
        // Exactly 63 characters.
        let name = "a".repeat(63);
        let result = validate_dns1123(&name).unwrap();
        assert_eq!(result, Dns1123Result::Valid(name));
    }

    #[test]
    fn dns1123_truncates_long_name() {
        // 70 alphanumeric characters.
        let name = "a".repeat(70);
        let result = validate_dns1123(&name).unwrap();
        assert!(matches!(result, Dns1123Result::Truncated(_)));
        assert_eq!(result.name().len(), 63);
    }

    #[test]
    fn dns1123_truncation_strips_trailing_hyphen() {
        // Build a name where position 63 would produce a trailing hyphen.
        // "aaa...aaa-bbb" where the hyphen falls at position 63.
        let prefix = "a".repeat(62);
        let name = format!("{prefix}-bbbbbbbbb");
        let result = validate_dns1123(&name).unwrap();
        assert!(matches!(result, Dns1123Result::Truncated(_)));
        assert!(!result.name().ends_with('-'));
    }

    #[test]
    fn dns1123_strips_invalid_characters() {
        let result = validate_dns1123("my@app!name").unwrap();
        assert_eq!(result, Dns1123Result::Valid("myappname".into()));
    }

    #[test]
    fn dns1123_strips_leading_trailing_hyphens() {
        let result = validate_dns1123("-my-app-").unwrap();
        assert_eq!(result, Dns1123Result::Valid("my-app".into()));
    }

    #[test]
    fn dns1123_uppercase_lowered() {
        let result = validate_dns1123("MyApp").unwrap();
        assert_eq!(result, Dns1123Result::Valid("myapp".into()));
    }

    #[test]
    fn dns1123_uppercase_snake_case() {
        let result = validate_dns1123("My_App").unwrap();
        assert_eq!(result, Dns1123Result::Valid("my-app".into()));
    }

    #[test]
    fn dns1123_empty_name_is_error() {
        assert!(validate_dns1123("").is_err());
    }

    #[test]
    fn dns1123_only_invalid_chars_is_error() {
        assert!(validate_dns1123("@#$%").is_err());
    }

    #[test]
    fn dns1123_only_hyphens_is_error() {
        assert!(validate_dns1123("---").is_err());
    }
}
