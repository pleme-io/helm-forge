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

#[cfg(feature = "backend")]
pub mod chart_gen;
#[cfg(feature = "backend")]
pub mod config;
#[cfg(feature = "backend")]
pub mod fluxcd_gen;
#[cfg(feature = "backend")]
pub mod helm_ast;
#[cfg(feature = "backend")]
pub mod helm_backend;
#[cfg(feature = "backend")]
pub mod model;
#[cfg(feature = "backend")]
pub mod naming;
#[cfg(feature = "backend")]
pub mod schema_gen;
#[cfg(feature = "backend")]
pub mod template_gen;
#[cfg(feature = "backend")]
pub mod test_gen;
#[cfg(feature = "backend")]
pub mod traits;
#[cfg(feature = "backend")]
pub mod type_map;
#[cfg(feature = "backend")]
pub mod values_gen;

// Core types
#[cfg(feature = "backend")]
pub use config::HelmConfig;
#[cfg(feature = "backend")]
pub use fluxcd_gen::{DefaultFluxCdGenerator, FluxCdConfig, FluxCdGenerator};
#[cfg(feature = "backend")]
pub use helm_backend::{HelmBackend, HelmBackendBuilder};
#[cfg(feature = "backend")]
pub use naming::{Dns1123Result, HelmNaming, validate_dns1123};

// Generator traits (for mockability and DI)
#[cfg(feature = "backend")]
pub use traits::{
    AttributeFilter, ChartGenerator, DefaultAttributeFilter, DefaultChartGenerator,
    DefaultSchemaGenerator, DefaultTemplateGenerator, DefaultTestFileGenerator,
    DefaultValuesGenerator, GenerationStage, SchemaGenerator, TemplateGenerator,
    TestFileGenerator, ValuesGenerator,
};

// Central data structures (serde-based YAML models + Helm template AST)
#[cfg(feature = "backend")]
pub use helm_ast::{HelmNode, PipeFilter, Trim, render as render_helm};
#[cfg(feature = "backend")]
pub use model::{
    AlertingConfig, ChartDependency, ChartYaml, ImageConfig, MonitoringConfig, PortConfig,
    ResourceQuantity, ResourcesConfig, ServiceConfig, ServicePort, ToggleConfig, ValuesYaml,
};

// Standalone generator functions (direct use without traits)
#[cfg(feature = "backend")]
pub use chart_gen::{generate_chart_yaml, generate_chart_yaml_with_config};
#[cfg(feature = "backend")]
pub use fluxcd_gen::{generate_helmrelease, generate_kustomization};
#[cfg(feature = "backend")]
pub use schema_gen::generate_values_schema;
#[cfg(feature = "backend")]
pub use template_gen::{
    generate_configmap_template, generate_deployment_template, generate_helpers_tpl,
    generate_hpa_template, generate_networkpolicy_template, generate_pdb_template,
    generate_podmonitor_template, generate_prometheusrule_template, generate_secret_template,
    generate_service_template, generate_serviceaccount_template,
    generate_servicemonitor_template,
};
#[cfg(feature = "backend")]
pub use test_gen::generate_deployment_test;
#[cfg(feature = "backend")]
pub use type_map::iac_type_to_json_schema;
#[cfg(feature = "backend")]
pub use values_gen::{generate_values_yaml, generate_values_yaml_with_config};

#[cfg(feature = "values-schema")]
pub mod values_schema;
