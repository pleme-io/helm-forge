//! Helm chart generator backend for [`iac_forge`].
//!
//! Implements the [`Backend`](iac_forge::Backend) trait to generate complete Helm
//! charts from TOML resource specs. Each chart delegates to `pleme-lib` named
//! templates for deployment, service, networkpolicy, and other resources.
//!
//! # Usage
//!
//! ```rust,no_run
//! use helm_forge::{HelmBackend, HelmConfig};
//! use iac_forge::Backend;
//!
//! // Default configuration
//! let backend = HelmBackend::default();
//!
//! // Or customise pleme-lib version, chart version, resource defaults
//! let backend = HelmBackend::with_config(HelmConfig {
//!     lib_chart_version: "~0.5.0".into(),
//!     ..HelmConfig::default()
//! });
//! ```

pub mod chart_gen;
pub mod config;
pub mod helm_backend;
pub mod naming;
pub mod schema_gen;
pub mod template_gen;
pub mod test_gen;
pub mod traits;
pub mod type_map;
pub mod values_gen;

pub use chart_gen::generate_chart_yaml;
pub use config::HelmConfig;
pub use helm_backend::HelmBackend;
pub use naming::HelmNaming;
pub use schema_gen::generate_values_schema;
pub use template_gen::{
    generate_configmap_template, generate_deployment_template, generate_helpers_tpl,
    generate_hpa_template, generate_networkpolicy_template, generate_pdb_template,
    generate_podmonitor_template, generate_secret_template, generate_service_template,
    generate_serviceaccount_template, generate_servicemonitor_template,
};
pub use test_gen::generate_deployment_test;
pub use traits::{
    AttributeFilter, ChartGenerator, DefaultAttributeFilter, SchemaGenerator, TemplateGenerator,
    TestFileGenerator, ValuesGenerator,
};
pub use type_map::iac_type_to_json_schema;
pub use values_gen::generate_values_yaml;
