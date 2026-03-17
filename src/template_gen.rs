use iac_forge::{IacAttribute, IacResource, to_kebab_case};

/// Generate `_helpers.tpl` that delegates to pleme-lib.
#[must_use]
pub fn generate_helpers_tpl(resource: &IacResource) -> String {
    let n = to_kebab_case(&resource.name);
    let mut s = String::new();

    s.push_str("{{/*\n");
    s.push_str(&format!("Helpers for {n} — delegates to pleme-lib.\n"));
    s.push_str("*/}}\n\n");

    for (helper, lib_helper) in [
        ("name", "pleme-lib.name"),
        ("fullname", "pleme-lib.fullname"),
        ("labels", "pleme-lib.labels"),
        ("selectorLabels", "pleme-lib.selectorLabels"),
    ] {
        s.push_str(&format!("{{{{- define \"{n}.{helper}\" -}}}}\n"));
        s.push_str(&format!("{{{{- include \"{lib_helper}\" . -}}}}\n"));
        s.push_str("{{- end -}}\n\n");
    }

    s.trim_end().to_string()
    // Trim trailing newlines but keep one
    + "\n"
}

/// Generate a single-line template that delegates to a pleme-lib named template.
fn pleme_lib_delegate(template_name: &str) -> String {
    format!("{{{{- include \"pleme-lib.{template_name}\" . }}}}\n")
}

/// Generate `deployment.yaml` delegating to pleme-lib.
#[must_use]
pub fn generate_deployment_template() -> String {
    pleme_lib_delegate("deployment")
}

/// Generate `service.yaml` delegating to pleme-lib.
#[must_use]
pub fn generate_service_template() -> String {
    pleme_lib_delegate("service")
}

/// Generate `serviceaccount.yaml` delegating to pleme-lib.
#[must_use]
pub fn generate_serviceaccount_template() -> String {
    pleme_lib_delegate("serviceaccount")
}

/// Generate `servicemonitor.yaml` delegating to pleme-lib.
#[must_use]
pub fn generate_servicemonitor_template() -> String {
    pleme_lib_delegate("servicemonitor")
}

/// Generate `networkpolicy.yaml` delegating to pleme-lib.
#[must_use]
pub fn generate_networkpolicy_template() -> String {
    pleme_lib_delegate("networkpolicy")
}

/// Generate `pdb.yaml` delegating to pleme-lib.
#[must_use]
pub fn generate_pdb_template() -> String {
    pleme_lib_delegate("pdb")
}

/// Generate `hpa.yaml` delegating to pleme-lib.
#[must_use]
pub fn generate_hpa_template() -> String {
    pleme_lib_delegate("hpa")
}

/// Generate `podmonitor.yaml` delegating to pleme-lib.
#[must_use]
pub fn generate_podmonitor_template() -> String {
    pleme_lib_delegate("podmonitor")
}

/// Generate `configmap.yaml` for non-sensitive resource attributes.
#[must_use]
pub fn generate_configmap_template(resource: &IacResource) -> String {
    let config_attrs: Vec<&IacAttribute> = resource
        .attributes
        .iter()
        .filter(|a| !a.sensitive && !a.computed)
        .collect();

    if config_attrs.is_empty() {
        return String::new();
    }

    let n = to_kebab_case(&resource.name);
    let mut lines = Vec::new();

    lines.push("{{- if .Values.config }}".into());
    lines.push("apiVersion: v1".into());
    lines.push("kind: ConfigMap".into());
    lines.push("metadata:".into());
    lines.push(format!("  name: {{{{{{ include \"{n}.fullname\" . }}}}}}"));
    lines.push(format!(
        "  labels:"
    ));
    lines.push(format!(
        "    {{{{- include \"{n}.labels\" . | nindent 4 }}}}"
    ));
    lines.push("data:".into());

    for attr in &config_attrs {
        let key = attr.canonical_name.replace('_', "-");
        lines.push(format!(
            "  {key}: {{{{{{ .Values.config.{} | quote }}}}}}",
            attr.canonical_name
        ));
    }

    lines.push("{{- end }}".into());
    lines.push(String::new());

    lines.join("\n")
}

/// Generate `secret.yaml` for sensitive resource attributes.
#[must_use]
pub fn generate_secret_template(resource: &IacResource) -> String {
    let secret_attrs: Vec<&IacAttribute> = resource
        .attributes
        .iter()
        .filter(|a| a.sensitive && !a.computed)
        .collect();

    if secret_attrs.is_empty() {
        return String::new();
    }

    let n = to_kebab_case(&resource.name);
    let mut lines = Vec::new();

    lines.push("{{- if .Values.secrets }}".into());
    lines.push("apiVersion: v1".into());
    lines.push("kind: Secret".into());
    lines.push("metadata:".into());
    lines.push(format!("  name: {{{{{{ include \"{n}.fullname\" . }}}}}}"));
    lines.push("  labels:".into());
    lines.push(format!(
        "    {{{{- include \"{n}.labels\" . | nindent 4 }}}}"
    ));
    lines.push("type: Opaque".into());
    lines.push("stringData:".into());

    for attr in &secret_attrs {
        let key = attr.canonical_name.replace('_', "-");
        lines.push(format!(
            "  {key}: {{{{{{ .Values.secrets.{} | quote }}}}}}",
            attr.canonical_name
        ));
    }

    lines.push("{{- end }}".into());
    lines.push(String::new());

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use iac_forge::testing::test_resource;

    #[test]
    fn helpers_contains_chart_name_and_all_delegates() {
        let resource = test_resource("static_secret");
        let tpl = generate_helpers_tpl(&resource);
        assert!(tpl.contains("static-secret.name"));
        assert!(tpl.contains("static-secret.fullname"));
        assert!(tpl.contains("static-secret.labels"));
        assert!(tpl.contains("static-secret.selectorLabels"));
        assert!(tpl.contains("pleme-lib.name"));
        assert!(tpl.contains("pleme-lib.fullname"));
        assert!(tpl.contains("pleme-lib.labels"));
        assert!(tpl.contains("pleme-lib.selectorLabels"));
    }

    #[test]
    fn all_delegate_templates_reference_pleme_lib() {
        assert!(generate_deployment_template().contains("pleme-lib.deployment"));
        assert!(generate_service_template().contains("pleme-lib.service"));
        assert!(generate_serviceaccount_template().contains("pleme-lib.serviceaccount"));
        assert!(generate_servicemonitor_template().contains("pleme-lib.servicemonitor"));
        assert!(generate_networkpolicy_template().contains("pleme-lib.networkpolicy"));
        assert!(generate_pdb_template().contains("pleme-lib.pdb"));
        assert!(generate_hpa_template().contains("pleme-lib.hpa"));
        assert!(generate_podmonitor_template().contains("pleme-lib.podmonitor"));
    }

    #[test]
    fn configmap_includes_non_sensitive_attrs() {
        let resource = test_resource("static_secret");
        let tpl = generate_configmap_template(&resource);
        assert!(tpl.contains("ConfigMap"));
        assert!(tpl.contains(".Values.config"));
    }

    #[test]
    fn secret_includes_sensitive_attrs() {
        let resource = test_resource("static_secret");
        let tpl = generate_secret_template(&resource);
        assert!(tpl.contains("Secret"));
        assert!(tpl.contains(".Values.secrets"));
    }
}
