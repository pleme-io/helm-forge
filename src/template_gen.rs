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

    s.trim_end().to_string() + "\n"
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
    lines.push(format!("  name: {{{{ include \"{n}.fullname\" . }}}}"));
    lines.push("  labels:".into());
    lines.push(format!(
        "    {{{{- include \"{n}.labels\" . | nindent 4 }}}}"
    ));
    lines.push("data:".into());

    for attr in &config_attrs {
        let key = attr.canonical_name.replace('_', "-");
        lines.push(format!(
            "  {key}: {{{{ .Values.config.{} | quote }}}}",
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
    lines.push(format!("  name: {{{{ include \"{n}.fullname\" . }}}}"));
    lines.push("  labels:".into());
    lines.push(format!(
        "    {{{{- include \"{n}.labels\" . | nindent 4 }}}}"
    ));
    lines.push("type: Opaque".into());
    lines.push("stringData:".into());

    for attr in &secret_attrs {
        let key = attr.canonical_name.replace('_', "-");
        lines.push(format!(
            "  {key}: {{{{ .Values.secrets.{} | quote }}}}",
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
    use iac_forge::testing::{test_resource, test_resource_with_type};
    use iac_forge::IacType;

    #[test]
    fn helpers_contains_all_four_delegates() {
        let resource = test_resource("static_secret");
        let tpl = generate_helpers_tpl(&resource);
        for helper in ["name", "fullname", "labels", "selectorLabels"] {
            assert!(
                tpl.contains(&format!("static-secret.{helper}")),
                "missing define for {helper}"
            );
        }
        for lib in [
            "pleme-lib.name",
            "pleme-lib.fullname",
            "pleme-lib.labels",
            "pleme-lib.selectorLabels",
        ] {
            assert!(tpl.contains(lib), "missing include for {lib}");
        }
    }

    #[test]
    fn helpers_produces_valid_helm_syntax() {
        let resource = test_resource("test_res");
        let tpl = generate_helpers_tpl(&resource);
        // Must have balanced {{ and }} delimiters
        assert!(tpl.contains("{{- define \"test-res.name\" -}}"));
        assert!(tpl.contains("{{- include \"pleme-lib.name\" . -}}"));
        assert!(tpl.contains("{{- end -}}"));
        // Must NOT have triple braces
        assert!(!tpl.contains("{{{"), "triple open braces in helpers");
        assert!(!tpl.contains("}}}"), "triple close braces in helpers");
    }

    #[test]
    fn all_delegate_templates_have_valid_helm_syntax() {
        let delegates = [
            ("deployment", generate_deployment_template()),
            ("service", generate_service_template()),
            ("serviceaccount", generate_serviceaccount_template()),
            ("servicemonitor", generate_servicemonitor_template()),
            ("networkpolicy", generate_networkpolicy_template()),
            ("pdb", generate_pdb_template()),
            ("hpa", generate_hpa_template()),
            ("podmonitor", generate_podmonitor_template()),
        ];

        for (name, tpl) in &delegates {
            assert!(
                tpl.contains(&format!("pleme-lib.{name}")),
                "{name} missing pleme-lib reference"
            );
            assert!(
                tpl.starts_with("{{- include"),
                "{name} must start with Helm include"
            );
            assert!(
                tpl.trim_end().ends_with("}}"),
                "{name} must end with Helm closing braces"
            );
            assert!(
                !tpl.contains("{{{"),
                "{name} has triple open braces"
            );
            assert!(
                !tpl.contains("}}}"),
                "{name} has triple close braces"
            );
        }
    }

    #[test]
    fn configmap_has_valid_helm_syntax() {
        let resource = test_resource("static_secret");
        let tpl = generate_configmap_template(&resource);

        assert!(tpl.contains("kind: ConfigMap"));
        assert!(tpl.contains("{{- if .Values.config }}"));
        assert!(tpl.contains("{{ include \"static-secret.fullname\" . }}"));
        assert!(tpl.contains("{{- include \"static-secret.labels\" . | nindent 4 }}"));
        assert!(tpl.contains(".Values.config."));
        assert!(tpl.contains("| quote }}"));
        assert!(tpl.contains("{{- end }}"));
        // No triple braces
        assert!(!tpl.contains("{{{"), "triple open braces in configmap");
        assert!(!tpl.contains("}}}"), "triple close braces in configmap");
    }

    #[test]
    fn secret_has_valid_helm_syntax() {
        let resource = test_resource("static_secret");
        let tpl = generate_secret_template(&resource);

        assert!(tpl.contains("kind: Secret"));
        assert!(tpl.contains("type: Opaque"));
        assert!(tpl.contains("{{- if .Values.secrets }}"));
        assert!(tpl.contains("{{ include \"static-secret.fullname\" . }}"));
        assert!(tpl.contains("{{- include \"static-secret.labels\" . | nindent 4 }}"));
        assert!(tpl.contains(".Values.secrets."));
        assert!(tpl.contains("| quote }}"));
        assert!(tpl.contains("{{- end }}"));
        assert!(!tpl.contains("{{{"), "triple open braces in secret");
        assert!(!tpl.contains("}}}"), "triple close braces in secret");
    }

    #[test]
    fn configmap_empty_when_no_non_sensitive_attrs() {
        // All attributes are sensitive → no configmap
        let resource = test_resource_with_type("all_sensitive", "secret_val", IacType::String);
        // test_resource_with_type creates one attr; manually make it sensitive
        let mut r = resource;
        for attr in &mut r.attributes {
            attr.sensitive = true;
        }
        let tpl = generate_configmap_template(&r);
        assert!(tpl.is_empty());
    }

    #[test]
    fn secret_empty_when_no_sensitive_attrs() {
        let resource = test_resource_with_type("all_config", "plain_val", IacType::String);
        let tpl = generate_secret_template(&resource);
        assert!(tpl.is_empty());
    }

    #[test]
    fn configmap_keys_are_kebab_cased() {
        let resource = test_resource("test_res");
        let tpl = generate_configmap_template(&resource);
        // test_resource has canonical_name with underscores; configmap key should use hyphens
        assert!(!tpl.is_empty());
        // Should not have underscored keys in the data section
        for line in tpl.lines() {
            if line.starts_with("  ") && line.contains("| quote") {
                let key = line.trim().split(':').next().unwrap();
                assert!(!key.contains('_'), "configmap key {key} has underscore");
            }
        }
    }
}
