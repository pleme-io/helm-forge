use iac_forge::{IacAttribute, IacResource, to_kebab_case};

/// Generate `_helpers.tpl` that delegates to pleme-lib.
#[must_use]
pub fn generate_helpers_tpl(resource: &IacResource) -> String {
    let chart_name = to_kebab_case(&resource.name);
    format!(
        r#"{{{{/*
Helpers for {chart_name} — delegates to pleme-lib.
*/}}}}

{{{{- define "{chart_name}.name" -}}}}
{{{{- include "pleme-lib.name" . -}}}}
{{{{- end -}}}}

{{{{- define "{chart_name}.fullname" -}}}}
{{{{- include "pleme-lib.fullname" . -}}}}
{{{{- end -}}}}

{{{{- define "{chart_name}.labels" -}}}}
{{{{- include "pleme-lib.labels" . -}}}}
{{{{- end -}}}}

{{{{- define "{chart_name}.selectorLabels" -}}}}
{{{{- include "pleme-lib.selectorLabels" . -}}}}
{{{{- end -}}}}
"#
    )
}

/// Generate `deployment.yaml` delegating to pleme-lib.
#[must_use]
pub fn generate_deployment_template() -> String {
    String::from("{{- include \"pleme-lib.deployment\" . }}\n")
}

/// Generate `service.yaml` delegating to pleme-lib.
#[must_use]
pub fn generate_service_template() -> String {
    String::from("{{- include \"pleme-lib.service\" . }}\n")
}

/// Generate `serviceaccount.yaml` delegating to pleme-lib.
#[must_use]
pub fn generate_serviceaccount_template() -> String {
    String::from("{{- include \"pleme-lib.serviceaccount\" . }}\n")
}

/// Generate `servicemonitor.yaml` delegating to pleme-lib.
#[must_use]
pub fn generate_servicemonitor_template() -> String {
    String::from("{{- include \"pleme-lib.servicemonitor\" . }}\n")
}

/// Generate `networkpolicy.yaml` delegating to pleme-lib.
#[must_use]
pub fn generate_networkpolicy_template() -> String {
    String::from("{{- include \"pleme-lib.networkpolicy\" . }}\n")
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

    let chart_name = to_kebab_case(&resource.name);
    let mut lines = Vec::new();

    lines.push(format!(
        "{{{{- if .Values.config }}}}}"
    ));
    lines.push("apiVersion: v1".into());
    lines.push("kind: ConfigMap".into());
    lines.push("metadata:".into());
    lines.push(format!(
        "  name: {{{{{{ include \"{chart_name}.fullname\" . }}}}}}"
    ));
    lines.push(format!(
        "  labels:\n    {{{{- include \"{chart_name}.labels\" . | nindent 4 }}}}}}"
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

    let chart_name = to_kebab_case(&resource.name);
    let mut lines = Vec::new();

    lines.push(format!("{{{{- if .Values.secrets }}}}}}"));
    lines.push("apiVersion: v1".into());
    lines.push("kind: Secret".into());
    lines.push("metadata:".into());
    lines.push(format!(
        "  name: {{{{{{ include \"{chart_name}.fullname\" . }}}}}}"
    ));
    lines.push(format!(
        "  labels:\n    {{{{- include \"{chart_name}.labels\" . | nindent 4 }}}}}}"
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
    fn helpers_contains_chart_name() {
        let resource = test_resource("static_secret");
        let tpl = generate_helpers_tpl(&resource);
        assert!(tpl.contains("static-secret"));
        assert!(tpl.contains("pleme-lib.name"));
    }

    #[test]
    fn deployment_delegates_to_pleme_lib() {
        let tpl = generate_deployment_template();
        assert!(tpl.contains("pleme-lib.deployment"));
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
