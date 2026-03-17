//! Generator traits for mockability and substitution.
//!
//! Each generation step is behind a trait so consumers can:
//! - Mock individual generators in tests
//! - Replace generators with custom implementations
//! - Inspect generation plans without producing output

use iac_forge::ir::{IacAttribute, IacResource};

/// Filter resource attributes into config (non-sensitive) and secret (sensitive) groups.
pub trait AttributeFilter: std::fmt::Debug {
    /// Attributes that should appear in ConfigMap and `config:` values section.
    fn config_attributes<'a>(&self, resource: &'a IacResource) -> Vec<&'a IacAttribute>;

    /// Attributes that should appear in Secret and `secrets:` values section.
    fn secret_attributes<'a>(&self, resource: &'a IacResource) -> Vec<&'a IacAttribute>;
}

/// Default filter: non-sensitive + non-computed → config, sensitive + non-computed → secret.
#[derive(Debug, Clone, Copy)]
pub struct DefaultAttributeFilter;

impl AttributeFilter for DefaultAttributeFilter {
    fn config_attributes<'a>(&self, resource: &'a IacResource) -> Vec<&'a IacAttribute> {
        resource
            .attributes
            .iter()
            .filter(|a| !a.sensitive && !a.computed)
            .collect()
    }

    fn secret_attributes<'a>(&self, resource: &'a IacResource) -> Vec<&'a IacAttribute> {
        resource
            .attributes
            .iter()
            .filter(|a| a.sensitive && !a.computed)
            .collect()
    }
}

/// Generates `Chart.yaml` content.
pub trait ChartGenerator: std::fmt::Debug {
    fn generate(&self, resource: &IacResource, provider_name: &str) -> String;
}

/// Generates `values.yaml` content.
pub trait ValuesGenerator: std::fmt::Debug {
    fn generate(&self, resource: &IacResource) -> String;
}

/// Generates `values.schema.json` content.
pub trait SchemaGenerator: std::fmt::Debug {
    fn generate(&self, resource: &IacResource) -> String;
}

/// Generates Helm template files.
pub trait TemplateGenerator: std::fmt::Debug {
    /// All delegate templates (deployment.yaml, service.yaml, etc.).
    fn delegate_templates(&self) -> Vec<(&'static str, String)>;

    /// `_helpers.tpl` for chart-specific helper definitions.
    fn helpers(&self, resource: &IacResource) -> String;

    /// ConfigMap template (empty string if no config attributes).
    fn configmap(&self, resource: &IacResource) -> String;

    /// Secret template (empty string if no secret attributes).
    fn secret(&self, resource: &IacResource) -> String;
}

/// Generates helm-unittest test files.
pub trait TestFileGenerator: std::fmt::Debug {
    fn generate(&self, resource: &IacResource) -> String;
}

#[cfg(test)]
mod tests {
    use super::*;
    use iac_forge::testing::{test_resource, test_resource_with_type, TestAttributeBuilder};
    use iac_forge::IacType;

    #[test]
    fn default_filter_separates_config_and_secrets() {
        let resource = test_resource("test");
        let filter = DefaultAttributeFilter;

        let config = filter.config_attributes(&resource);
        let secrets = filter.secret_attributes(&resource);

        // test_resource has: name (required+immutable), value (required+sensitive), tags (list)
        assert!(!config.is_empty(), "should have config attrs");
        assert!(!secrets.is_empty(), "should have secret attrs");

        // No overlap
        for c in &config {
            assert!(!c.sensitive, "config attr {} is sensitive", c.canonical_name);
        }
        for s in &secrets {
            assert!(s.sensitive, "secret attr {} is not sensitive", s.canonical_name);
        }
    }

    #[test]
    fn default_filter_excludes_computed() {
        let mut resource = test_resource_with_type("test", "computed_field", IacType::String);
        resource.attributes[0].computed = true;

        let filter = DefaultAttributeFilter;
        assert!(filter.config_attributes(&resource).is_empty());
        assert!(filter.secret_attributes(&resource).is_empty());
    }

    #[test]
    fn default_filter_empty_resource() {
        let mut resource = test_resource("empty");
        resource.attributes.clear();

        let filter = DefaultAttributeFilter;
        assert!(filter.config_attributes(&resource).is_empty());
        assert!(filter.secret_attributes(&resource).is_empty());
    }

    #[test]
    fn default_filter_only_sensitive() {
        let mut resource = test_resource_with_type("sens", "secret_val", IacType::String);
        resource.attributes[0].sensitive = true;

        let filter = DefaultAttributeFilter;
        assert!(filter.config_attributes(&resource).is_empty());
        assert_eq!(filter.secret_attributes(&resource).len(), 1);
    }

    #[test]
    fn default_filter_only_non_sensitive() {
        let resource = test_resource_with_type("plain", "plain_val", IacType::String);

        let filter = DefaultAttributeFilter;
        assert_eq!(filter.config_attributes(&resource).len(), 1);
        assert!(filter.secret_attributes(&resource).is_empty());
    }

    #[test]
    fn default_filter_mixed_attributes() {
        let mut resource = test_resource("mixed");
        // test_resource creates: name (not sensitive), value (sensitive), tags (not sensitive)
        // Add a computed attribute that should be excluded from both
        let computed = TestAttributeBuilder::new("output_id", IacType::String)
            .computed()
            .build();
        resource.attributes.push(computed);

        let filter = DefaultAttributeFilter;
        let config = filter.config_attributes(&resource);
        let secrets = filter.secret_attributes(&resource);

        // Computed should not appear in either
        assert!(!config.iter().any(|a| a.canonical_name == "output_id"));
        assert!(!secrets.iter().any(|a| a.canonical_name == "output_id"));
    }
}
