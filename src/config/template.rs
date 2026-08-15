//! YAML-safe templating for configuration files.
//!
//! # Why this exists
//!
//! Config files are rendered through a template engine before being parsed as
//! YAML, so values can be pulled from the environment:
//!
//! ```yaml
//! port: <%= get_env(name="PORT", default="5150") %>
//! ```
//!
//! The engine underneath is Tera, whose native delimiters are `{{ }}`/`{% %}`.
//! Those are **not safe to write in a YAML file**: `{` is a YAML flow-mapping
//! indicator, so `port: {{ get_env(...) }}` is not valid YAML at rest. The file
//! only ever parses because the template pass rewrites it first — which means
//! any tool that reads the file *as YAML* (prettier, yaml-language-server,
//! editors with format-on-save) mangles it into `{ { ... } }` and breaks
//! startup. See <https://github.com/loco-rs/loco/issues/1727>.
//!
//! `<` is not a YAML indicator character (YAML 1.2 §5.3), so `<%= ... %>` is an
//! ordinary plain scalar: the file is valid YAML *before* rendering, formatters
//! leave it alone, and round-tripping it through a YAML formatter cannot change
//! its meaning. This is the same trick Rails' ERB-in-`database.yml` has relied
//! on for two decades.
//!
//! This module translates the YAML-safe delimiters into Tera's own just before
//! rendering. Rendering stays *textual* — a value renders to a bare `5150`,
//! which YAML then types as an integer — so typed fields keep working.
//!
//! Legacy `{{ }}` templates still render, with a deprecation warning, so config
//! files written before this change keep working.

use crate::{Error, Result};

/// Opening delimiter of a YAML-safe template tag.
const OPEN: &str = "<%";
/// Closing delimiter of a YAML-safe template tag.
const CLOSE: &str = "%>";

/// Renders a configuration file's contents.
///
/// Accepts the YAML-safe delimiters (`<%= expr %>` for interpolation,
/// `<% stmt %>` for statements, `<%# text %>` for comments) and, for backward
/// compatibility, Tera's native `{{ }}`/`{% %}` delimiters.
///
/// # Errors
/// Returns an error if a tag is unterminated or if rendering fails.
pub fn render(content: &str) -> Result<String> {
    warn_legacy_delimiters(content, &mut std::io::stderr());

    let translated = to_tera_syntax(content)?;
    crate::tera::render_string(&translated, &serde_json::json!({}))
}

/// Writes the legacy-delimiter deprecation notice, if it applies.
///
/// Deliberately **not** `tracing::warn!`. Config is loaded before
/// `logger::init` runs (`cli.rs`: `H::load_config(..)` precedes
/// `logger::init::<H>(..)`), so at this point no subscriber exists and a
/// `warn!` goes nowhere. The warning was promised by both the CHANGELOG and
/// this module's own docs, and nobody could ever have seen it.
///
/// Takes its sink so the message can be asserted on; production passes
/// stderr.
fn warn_legacy_delimiters(content: &str, out: &mut impl std::io::Write) {
    if !has_legacy_delimiters(content) {
        return;
    }
    // Nothing useful to do if stderr is closed -- this is a courtesy notice,
    // not a reason to fail loading a config that renders fine.
    let _ = writeln!(
        out,
        "warning: this config uses the legacy `{{{{ }}}}` template delimiters, which are not \
         valid YAML and are rewritten by editors/formatters (see \
         https://github.com/loco-rs/loco/issues/1727). Use the YAML-safe form instead, e.g. \
         `<%= get_env(name=\"PORT\", default=\"5150\") %>`."
    );
}

/// Whether the source still uses Tera's native (YAML-unsafe) delimiters.
fn has_legacy_delimiters(content: &str) -> bool {
    content.contains("{{") || content.contains("{%")
}

/// Rewrites YAML-safe delimiters into the Tera delimiters they stand for.
///
/// | written in config | rendered as |
/// |---|---|
/// | `<%= expr %>`  | `{{ expr }}` |
/// | `<% stmt %>`   | `{% stmt %}` |
/// | `<%# text %>`  | `{# text #}` |
///
/// Everything outside a tag is copied verbatim, so an unrelated `%>` in a value
/// is untouched. Content that uses no `<%` tags is returned unchanged.
fn to_tera_syntax(content: &str) -> Result<String> {
    if !content.contains(OPEN) {
        return Ok(content.to_string());
    }

    let mut out = String::with_capacity(content.len());
    let mut rest = content;

    while let Some(start) = rest.find(OPEN) {
        out.push_str(&rest[..start]);
        let after_open = &rest[start + OPEN.len()..];

        let Some(end) = after_open.find(CLOSE) else {
            return Err(Error::Message(format!(
                "unterminated `{OPEN}` template tag in configuration: expected a closing `{CLOSE}`"
            )));
        };

        let (kind, body) = split_tag(&after_open[..end]);
        let (tera_open, tera_close) = kind.tera_delimiters();
        out.push_str(tera_open);
        out.push_str(body);
        out.push_str(tera_close);

        rest = &after_open[end + CLOSE.len()..];
    }

    out.push_str(rest);
    Ok(out)
}

/// The three tag flavors, mirroring Tera's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TagKind {
    /// `<%= expr %>` — interpolate a value.
    Expression,
    /// `<% stmt %>` — a statement/block.
    Statement,
    /// `<%# text %>` — a comment.
    Comment,
}

impl TagKind {
    /// The Tera delimiter pair this tag translates to.
    const fn tera_delimiters(self) -> (&'static str, &'static str) {
        match self {
            Self::Expression => ("{{", "}}"),
            Self::Statement => ("{%", "%}"),
            Self::Comment => ("{#", "#}"),
        }
    }
}

/// Splits a tag's inner text into its kind and body, based on the sigil that
/// immediately follows `<%`.
fn split_tag(inner: &str) -> (TagKind, &str) {
    match inner.split_at_checked(1) {
        Some(("=", body)) => (TagKind::Expression, body),
        Some(("#", body)) => (TagKind::Comment, body),
        _ => (TagKind::Statement, inner),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_the_three_tag_kinds() {
        assert_eq!(to_tera_syntax("a: <%= x %>").unwrap(), "a: {{ x }}");
        assert_eq!(to_tera_syntax("<% if x %>").unwrap(), "{% if x %}");
        assert_eq!(to_tera_syntax("<%# note %>").unwrap(), "{# note #}");
    }

    #[test]
    fn leaves_untagged_content_untouched() {
        // A bare `%>` or `{` in a value must not be disturbed.
        let src = "a: 100%\nb: \"a %> b\"\nc: {not_a_tag}";
        assert_eq!(to_tera_syntax(src).unwrap(), src);
    }

    #[test]
    fn translates_multiple_tags_on_one_line_and_across_lines() {
        assert_eq!(
            to_tera_syntax("uri: \"<%= a %>://<%= b %>\"\nport: <%= c %>").unwrap(),
            "uri: \"{{ a }}://{{ b }}\"\nport: {{ c }}"
        );
    }

    #[test]
    fn unterminated_tag_is_a_clear_error() {
        let err = to_tera_syntax("port: <%= get_env(name=\"PORT\")").unwrap_err();
        assert!(err.to_string().contains("unterminated"), "got: {err}");
    }

    #[test]
    fn renders_env_lookup_with_default() {
        let out =
            render("port: <%= get_env(name=\"NOPE_UNSET_VAR\", default=\"5150\") %>").unwrap();
        assert_eq!(out, "port: 5150");
    }

    #[test]
    fn rendered_output_is_bare_so_yaml_keeps_the_type() {
        // The whole point of textual substitution: an int stays an int.
        let out =
            render("port: <%= get_env(name=\"NOPE_UNSET_VAR\", default=\"5150\") %>").unwrap();
        let v: serde_yaml::Value = serde_yaml::from_str(&out).unwrap();
        assert!(
            v["port"].is_i64(),
            "expected an integer, got: {:?}",
            v["port"]
        );
    }

    /// A statement tag must actually drive rendering, not just translate — this
    /// is how a config switches a block on an environment variable.
    #[test]
    fn statement_tags_conditionally_render_config() {
        let src = "\
<% if get_env(name=\"NOPE_UNSET_VAR\", default=\"off\") == \"on\" %>\
cache: {kind: Redis}\
<% else %>\
cache: {kind: Null}\
<% endif %>";
        assert_eq!(render(src).unwrap(), "cache: {kind: Null}");
    }

    /// Comment tags must disappear from the rendered YAML entirely.
    #[test]
    fn comment_tags_are_stripped() {
        let out = render("port: 5150<%# an internal note %>").unwrap();
        assert_eq!(out, "port: 5150");
    }

    #[test]
    fn reads_a_set_environment_variable() {
        // SAFETY: single-threaded test; the variable name is unique to it.
        unsafe { std::env::set_var("LOCO_CONFIG_TEST_PORT", "9999") };
        let out = render("port: <%= get_env(name=\"LOCO_CONFIG_TEST_PORT\") %>").unwrap();
        assert_eq!(out, "port: 9999");
        // SAFETY: as above.
        unsafe { std::env::remove_var("LOCO_CONFIG_TEST_PORT") };
    }

    /// A variable that is *set but empty* must win over the default — matching
    /// shell semantics, and the reason `default` is a fallback for "unset", not
    /// for "empty".
    #[test]
    fn an_empty_environment_variable_is_still_a_value() {
        // SAFETY: single-threaded test; the variable name is unique to it.
        unsafe { std::env::set_var("LOCO_CONFIG_TEST_EMPTY", "") };
        let out = render(
            "host: \"<%= get_env(name=\"LOCO_CONFIG_TEST_EMPTY\", default=\"fallback\") %>\"",
        )
        .unwrap();
        assert_eq!(out, "host: \"\"");
        // SAFETY: as above.
        unsafe { std::env::remove_var("LOCO_CONFIG_TEST_EMPTY") };
    }

    /// An unset variable with no `default` must fail loudly rather than render
    /// an empty value into the config.
    #[test]
    fn missing_variable_without_default_is_an_error() {
        assert!(render("port: <%= get_env(name=\"LOCO_CONFIG_NOPE\") %>").is_err());
    }

    #[test]
    fn legacy_tera_delimiters_still_render() {
        let out = render("port: {{ get_env(name=\"NOPE_UNSET_VAR\", default=\"5150\") }}").unwrap();
        assert_eq!(out, "port: 5150");
    }

    #[test]
    fn detects_legacy_delimiters() {
        assert!(has_legacy_delimiters("a: {{ x }}"));
        assert!(has_legacy_delimiters("{% if x %}"));
        assert!(!has_legacy_delimiters("a: <%= x %>"));
        assert!(!has_legacy_delimiters("a: plain"));
    }

    /// The deprecation notice has to actually come out somewhere. It used to
    /// be a `tracing::warn!` emitted during `load_config`, which runs before
    /// `logger::init` — so no subscriber existed and the warning the
    /// CHANGELOG promised was unreachable.
    #[test]
    fn the_legacy_delimiter_warning_is_written_out() {
        let mut out = Vec::new();
        warn_legacy_delimiters("port: {{ get_env(name=\"PORT\") }}", &mut out);

        let written = String::from_utf8(out).expect("the notice is valid UTF-8");
        assert!(
            written.contains("legacy") && written.contains("1727"),
            "the notice should name the problem and point at the issue: {written}"
        );
        assert!(
            written.contains("<%="),
            "the notice should show the replacement form: {written}"
        );
    }

    #[test]
    fn a_yaml_safe_config_warns_about_nothing() {
        let mut out = Vec::new();
        warn_legacy_delimiters("port: <%= get_env(name=\"PORT\") %>", &mut out);
        assert!(out.is_empty(), "nothing to deprecate, nothing to say");
    }

    /// Regression guard for <https://github.com/loco-rs/loco/issues/1727>: a
    /// config using the YAML-safe delimiters must be parseable as YAML
    /// *before* it is rendered, so editors and formatters cannot corrupt it.
    #[test]
    fn unrendered_config_is_valid_yaml() {
        let src = r#"
server:
  port: <%= get_env(name="PORT", default="5150") %>
  binding: <%= get_env(name="BINDING", default="localhost") %>
database:
  uri: <%= get_env(name="DATABASE_URL", default="postgres://localhost/app") %>
"#;
        let parsed: serde_yaml::Value =
            serde_yaml::from_str(src).expect("unrendered config must be valid YAML");
        // Each templated value is an ordinary string scalar, so a formatter has
        // nothing to restructure.
        assert!(parsed["server"]["port"].is_string());

        // The legacy form, by contrast, is read as *structure*: `{` opens a YAML
        // flow mapping, so the template becomes a nested mapping rather than
        // text. That is precisely why formatters rewrite it to `{ { ... } }`
        // and break startup. (Some YAML parsers reject it outright; the ones
        // that accept it still do not see a string.)
        let legacy = "port: {{ get_env(name=\"PORT\", default=\"5150\") }}";
        let legacy_parsed = serde_yaml::from_str::<serde_yaml::Value>(legacy);
        assert!(
            legacy_parsed.map_or(true, |v| !v["port"].is_string()),
            "legacy delimiters must not be seen as a plain string by a YAML parser"
        );
    }

    /// A rendered config and a *formatter-normalized* one must mean the same
    /// thing — the property that makes the YAML-safe delimiters immune to
    /// format-on-save.
    #[test]
    fn survives_yaml_formatter_round_trip() {
        let src = "server:\n  port: <%= get_env(name=\"NOPE_UNSET_VAR\", default=\"5150\") %>\n";

        // Simulate a formatter: parse as YAML, re-emit, and re-parse.
        let as_value: serde_yaml::Value = serde_yaml::from_str(src).unwrap();
        let reformatted = serde_yaml::to_string(&as_value).unwrap();

        // Rendering either form yields the same config.
        let from_original = render(src).unwrap();
        let from_reformatted = render(&reformatted).unwrap();

        let a: serde_yaml::Value = serde_yaml::from_str(&from_original).unwrap();
        let b: serde_yaml::Value = serde_yaml::from_str(&from_reformatted).unwrap();
        assert_eq!(a, b, "formatting the config changed its meaning");
        assert_eq!(a["server"]["port"], serde_yaml::Value::from(5150));
    }
}
