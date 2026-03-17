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
//! // Custom config
//! let backend = HelmBackend::with_config(HelmConfig {
//!     lib_chart_version: "~0.5.0".into(),
//!     ..HelmConfig::default()
//! });
//!
//! // Builder with mock injection
//! let backend = HelmBackend::builder()
//!     .config(HelmConfig::default())
//!     .build();
//! ```

pub mod chart_gen;
pub mod config;
pub mod fluxcd_gen;
pub mod helm_ast;
pub mod helm_backend;
pub mod model;
pub mod naming;
pub mod schema_gen;
pub mod template_gen;
pub mod test_gen;
pub mod traits;
pub mod type_map;
pub mod values_gen;

// Core types
pub use config::HelmConfig;
pub use fluxcd_gen::{DefaultFluxCdGenerator, FluxCdConfig, FluxCdGenerator};
pub use helm_backend::{HelmBackend, HelmBackendBuilder};
pub use naming::{Dns1123Result, HelmNaming, validate_dns1123};

// Generator traits (for mockability and DI)
pub use traits::{
    AttributeFilter, ChartGenerator, DefaultAttributeFilter, DefaultChartGenerator,
    DefaultSchemaGenerator, DefaultTemplateGenerator, DefaultTestFileGenerator,
    DefaultValuesGenerator, GenerationStage, SchemaGenerator, TemplateGenerator,
    TestFileGenerator, ValuesGenerator,
};

// Central data structures (serde-based YAML models + Helm template AST)
pub use helm_ast::{HelmNode, PipeFilter, Trim, render as render_helm};
pub use model::{
    AlertingConfig, ChartDependency, ChartYaml, ImageConfig, MonitoringConfig, ResourceQuantity,
    ResourcesConfig, ToggleConfig, ValuesYaml,
};

// Standalone generator functions (direct use without traits)
pub use chart_gen::{generate_chart_yaml, generate_chart_yaml_with_config};
pub use fluxcd_gen::{generate_helmrelease, generate_kustomization};
pub use schema_gen::generate_values_schema;
pub use template_gen::{
    generate_configmap_template, generate_deployment_template, generate_helpers_tpl,
    generate_hpa_template, generate_networkpolicy_template, generate_pdb_template,
    generate_podmonitor_template, generate_prometheusrule_template, generate_secret_template,
    generate_service_template, generate_serviceaccount_template,
    generate_servicemonitor_template,
};
pub use test_gen::generate_deployment_test;
pub use type_map::iac_type_to_json_schema;
pub use values_gen::{generate_values_yaml, generate_values_yaml_with_config};
