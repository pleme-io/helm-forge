//! Helm template AST — typed representation of Go template syntax.
//!
//! Models the `{{ }}` / `{{- }}` Helm template language as a Rust enum,
//! enabling type-safe template generation. Templates are built as
//! `Vec<HelmNode>` and rendered to `String` via [`render`].
//!
//! # Example
//!
//! ```rust
//! use helm_forge::helm_ast::{HelmNode, render};
//!
//! let nodes = vec![
//!     HelmNode::include("pleme-lib.deployment", "."),
//! ];
//! let output = render(&nodes);
//! assert!(output.contains("pleme-lib.deployment"));
//! ```

use std::fmt;

/// A node in a Helm template document (mixed YAML + Go template syntax).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelmNode {
    /// Raw text/YAML content passed through verbatim.
    Text(String),

    /// `{{ include "template-name" <context> }}`
    Include {
        template: String,
        context: String,
        trim: Trim,
    },

    /// `{{ .Values.path.to.field }}`
    ValueRef {
        path: String,
        pipeline: Vec<PipeFilter>,
        trim: Trim,
    },

    /// `{{- if <condition> }}` ... `{{- end }}`
    If {
        condition: String,
        body: Vec<HelmNode>,
        else_body: Option<Vec<HelmNode>>,
    },

    /// `{{- range <over> }}` ... `{{- end }}`
    Range {
        over: String,
        body: Vec<HelmNode>,
    },

    /// `{{- define "<name>" -}}` ... `{{- end -}}`
    Define {
        name: String,
        body: Vec<HelmNode>,
    },

    /// `{{/* comment */}}`
    Comment(String),
}

/// Pipeline filters applied to an expression: `| nindent 4`, `| quote`, etc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipeFilter {
    Quote,
    Nindent(u32),
    Indent(u32),
    ToYaml,
    Upper,
    Lower,
    Default(String),
    Trim,
}

/// Whitespace trimming mode for template delimiters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trim {
    /// `{{ expr }}` — no trimming.
    None,
    /// `{{- expr }}` — trim left whitespace.
    Left,
    /// `{{ expr -}}` — trim right whitespace.
    Right,
    /// `{{- expr -}}` — trim both sides.
    Both,
}

// ── Convenience constructors ────────────────────────────────────────────────

impl HelmNode {
    /// `{{- include "<template>" <context> }}`
    #[must_use]
    pub fn include(template: &str, context: &str) -> Self {
        Self::Include {
            template: template.into(),
            context: context.into(),
            trim: Trim::Left,
        }
    }

    /// `{{ .Values.<path> | quote }}`
    #[must_use]
    pub fn value_quoted(path: &str) -> Self {
        Self::ValueRef {
            path: format!(".Values.{path}"),
            pipeline: vec![PipeFilter::Quote],
            trim: Trim::None,
        }
    }

    /// `{{ .Values.<path> }}`
    #[must_use]
    pub fn value_ref(path: &str) -> Self {
        Self::ValueRef {
            path: format!(".Values.{path}"),
            pipeline: vec![],
            trim: Trim::None,
        }
    }

    /// `{{- if .Values.<path> }}` ... `{{- end }}`
    #[must_use]
    pub fn if_values(path: &str, body: Vec<Self>) -> Self {
        Self::If {
            condition: format!(".Values.{path}"),
            body,
            else_body: None,
        }
    }

    /// Raw YAML text.
    #[must_use]
    pub fn text(s: &str) -> Self {
        Self::Text(s.into())
    }
}

// ── Rendering ───────────────────────────────────────────────────────────────

/// Render a sequence of `HelmNode`s to a Helm template string.
#[must_use]
pub fn render(nodes: &[HelmNode]) -> String {
    let mut out = String::new();
    for node in nodes {
        render_node(node, &mut out);
    }
    out
}

fn render_node(node: &HelmNode, out: &mut String) {
    match node {
        HelmNode::Text(text) => out.push_str(text),

        HelmNode::Include {
            template,
            context,
            trim,
        } => {
            let (open, close) = delimiters(*trim);
            out.push_str(&format!("{open} include \"{template}\" {context} {close}"));
        }

        HelmNode::ValueRef {
            path,
            pipeline,
            trim,
        } => {
            let (open, close) = delimiters(*trim);
            let mut expr = path.clone();
            for filter in pipeline {
                expr.push_str(&format!(" | {}", render_filter(filter)));
            }
            out.push_str(&format!("{open} {expr} {close}"));
        }

        HelmNode::If {
            condition,
            body,
            else_body,
        } => {
            out.push_str(&format!("{{{{- if {condition} }}}}\n"));
            for child in body {
                render_node(child, out);
            }
            if let Some(else_nodes) = else_body {
                out.push_str("{{- else }}\n");
                for child in else_nodes {
                    render_node(child, out);
                }
            }
            out.push_str("{{- end }}");
        }

        HelmNode::Range { over, body } => {
            out.push_str(&format!("{{{{- range {over} }}}}\n"));
            for child in body {
                render_node(child, out);
            }
            out.push_str("{{- end }}");
        }

        HelmNode::Define { name, body } => {
            out.push_str(&format!("{{{{- define \"{name}\" -}}}}\n"));
            for child in body {
                render_node(child, out);
            }
            out.push_str("{{- end -}}");
        }

        HelmNode::Comment(text) => {
            out.push_str(&format!("{{{{/* {text} */}}}}"));
        }
    }
}

fn delimiters(trim: Trim) -> (&'static str, &'static str) {
    match trim {
        Trim::None => ("{{", "}}"),
        Trim::Left => ("{{-", "}}"),
        Trim::Right => ("{{", "-}}"),
        Trim::Both => ("{{-", "-}}"),
    }
}

fn render_filter(filter: &PipeFilter) -> String {
    match filter {
        PipeFilter::Quote => "quote".into(),
        PipeFilter::Nindent(n) => format!("nindent {n}"),
        PipeFilter::Indent(n) => format!("indent {n}"),
        PipeFilter::ToYaml => "toYaml".into(),
        PipeFilter::Upper => "upper".into(),
        PipeFilter::Lower => "lower".into(),
        PipeFilter::Default(v) => format!("default \"{v}\""),
        PipeFilter::Trim => "trim".into(),
    }
}

impl fmt::Display for HelmNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = String::new();
        render_node(self, &mut buf);
        f.write_str(&buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_include() {
        let node = HelmNode::include("pleme-lib.deployment", ".");
        let s = render(&[node]);
        assert_eq!(s, "{{- include \"pleme-lib.deployment\" . }}");
    }

    #[test]
    fn render_value_ref() {
        let node = HelmNode::value_ref("config.name");
        let s = render(&[node]);
        assert_eq!(s, "{{ .Values.config.name }}");
    }

    #[test]
    fn render_value_quoted() {
        let node = HelmNode::value_quoted("secrets.api_key");
        let s = render(&[node]);
        assert_eq!(s, "{{ .Values.secrets.api_key | quote }}");
    }

    #[test]
    fn render_if_block() {
        let node = HelmNode::if_values("config", vec![HelmNode::text("hello\n")]);
        let s = render(&[node]);
        assert!(s.contains("{{- if .Values.config }}"));
        assert!(s.contains("hello"));
        assert!(s.contains("{{- end }}"));
    }

    #[test]
    fn render_if_else() {
        let node = HelmNode::If {
            condition: ".Values.enabled".into(),
            body: vec![HelmNode::text("yes\n")],
            else_body: Some(vec![HelmNode::text("no\n")]),
        };
        let s = render(&[node]);
        assert!(s.contains("{{- if .Values.enabled }}"));
        assert!(s.contains("yes"));
        assert!(s.contains("{{- else }}"));
        assert!(s.contains("no"));
        assert!(s.contains("{{- end }}"));
    }

    #[test]
    fn render_define() {
        let node = HelmNode::Define {
            name: "mychart.name".into(),
            body: vec![HelmNode::include("pleme-lib.name", ".")],
        };
        let s = render(&[node]);
        assert!(s.contains("{{- define \"mychart.name\" -}}"));
        assert!(s.contains("pleme-lib.name"));
        assert!(s.contains("{{- end -}}"));
    }

    #[test]
    fn render_comment() {
        let node = HelmNode::Comment("This is a comment".into());
        let s = render(&[node]);
        assert_eq!(s, "{{/* This is a comment */}}");
    }

    #[test]
    fn render_range() {
        let node = HelmNode::Range {
            over: ".Values.items".into(),
            body: vec![HelmNode::text("- item\n")],
        };
        let s = render(&[node]);
        assert!(s.contains("{{- range .Values.items }}"));
        assert!(s.contains("- item"));
        assert!(s.contains("{{- end }}"));
    }

    #[test]
    fn render_pipeline_filters() {
        let node = HelmNode::ValueRef {
            path: ".Values.data".into(),
            pipeline: vec![PipeFilter::ToYaml, PipeFilter::Nindent(4)],
            trim: Trim::Left,
        };
        let s = render(&[node]);
        assert_eq!(s, "{{- .Values.data | toYaml | nindent 4 }}");
    }

    #[test]
    fn trim_modes() {
        assert_eq!(delimiters(Trim::None), ("{{", "}}"));
        assert_eq!(delimiters(Trim::Left), ("{{-", "}}"));
        assert_eq!(delimiters(Trim::Right), ("{{", "-}}"));
        assert_eq!(delimiters(Trim::Both), ("{{-", "-}}"));
    }

    #[test]
    fn display_trait() {
        let node = HelmNode::include("pleme-lib.service", ".");
        assert_eq!(format!("{node}"), "{{- include \"pleme-lib.service\" . }}");
    }

    #[test]
    fn no_triple_braces_in_any_render() {
        let nodes = vec![
            HelmNode::include("chart.name", "."),
            HelmNode::value_ref("config.key"),
            HelmNode::value_quoted("secrets.val"),
            HelmNode::if_values("enabled", vec![HelmNode::text("ok\n")]),
            HelmNode::Comment("comment".into()),
            HelmNode::Define {
                name: "helper".into(),
                body: vec![HelmNode::text("body")],
            },
        ];
        let s = render(&nodes);
        assert!(!s.contains("{{{"), "triple open braces found");
        assert!(!s.contains("}}}"), "triple close braces found");
    }

    #[test]
    fn complex_template_composition() {
        let nodes = vec![
            HelmNode::if_values("config", vec![
                HelmNode::text("apiVersion: v1\n"),
                HelmNode::text("kind: ConfigMap\n"),
                HelmNode::text("metadata:\n"),
                HelmNode::text("  name: "),
                HelmNode::Include {
                    template: "mychart.fullname".into(),
                    context: ".".into(),
                    trim: Trim::None,
                },
                HelmNode::text("\n"),
                HelmNode::text("data:\n"),
                HelmNode::text("  key: "),
                HelmNode::value_quoted("config.key"),
                HelmNode::text("\n"),
            ]),
        ];
        let s = render(&nodes);
        assert!(s.contains("kind: ConfigMap"));
        assert!(s.contains("{{ include \"mychart.fullname\" . }}"));
        assert!(s.contains("{{ .Values.config.key | quote }}"));
        assert!(s.contains("{{- end }}"));
    }
}
