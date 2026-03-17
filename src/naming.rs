use iac_forge::backend::{ArtifactKind, NamingConvention};
use iac_forge::to_kebab_case;

/// Helm chart naming convention.
///
/// Resource names are kebab-cased for chart directory names.
/// Field names are preserved in their canonical (snake_case) form for values.yaml.
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
        assert_eq!(naming.field_name("bound-aws-account-id"), "bound_aws_account_id");
    }

    #[test]
    fn file_name_resource() {
        let naming = HelmNaming;
        assert_eq!(
            naming.file_name("static_secret", &ArtifactKind::Resource),
            "charts/static-secret/Chart.yaml"
        );
    }
}
