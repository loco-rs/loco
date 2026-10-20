//! `cargo loco generate oauth <provider>`: OAuth sign-in for an app.
//!
//! Renders the controller (once — a second provider reuses it), then edits
//! what a template cannot express as an injection: the provider's stanza in
//! each `config/*.yaml`, and the `auth_oauth` feature on `loco-rs` in
//! `Cargo.toml`. Every edit is planned before any is written, so a failure
//! leaves the app untouched.

use std::{fs, path::Path};

use serde_json::json;

use super::{AppInfo, Error, GenerateResults, Result};
use crate::RRgen;

/// The environments a generated app has, and how each gets its values.
const ENVIRONMENTS: [&str; 3] = ["development", "test", "production"];

/// Generates OAuth sign-in for `provider`.
///
/// # Errors
///
/// When `provider` is not a valid name, is already configured, or a file
/// cannot be read or written.
pub fn generate(rrgen: &RRgen, provider: &str, appinfo: &AppInfo) -> Result<GenerateResults> {
    validate_name(provider)?;
    let root = &appinfo.working_dir;

    let mut writes = Vec::new();
    for env in ENVIRONMENTS {
        let path = root.join("config").join(format!("{env}.yaml"));
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        let updated = add_provider(&content, provider, env)
            .map_err(|e| Error::Message(format!("config/{env}.yaml: {e}")))?;
        writes.push((path, updated));
    }
    if writes.is_empty() {
        return Err(Error::Message(
            "no config/{development,test,production}.yaml found; run this from the app root"
                .to_string(),
        ));
    }

    let cargo_toml = root.join("Cargo.toml");
    let mut messages = vec![];
    match fs::read_to_string(&cargo_toml)
        .ok()
        .and_then(|c| enable_feature(&c, "auth_oauth"))
    {
        Some(Edit::Changed(updated)) => writes.push((cargo_toml, updated)),
        Some(Edit::Unchanged) => {}
        None => messages.push(
            "Enable the `auth_oauth` feature on `loco-rs` in Cargo.toml: \
             loco-rs = { ..., features = [\"auth_oauth\"] }"
                .to_string(),
        ),
    }

    let mut results = crate::render_template(
        rrgen,
        Path::new("oauth"),
        &json!({"provider": provider, "pkg_name": appinfo.app_name}),
    )?;
    for (path, content) in writes {
        fs::write(path, content)?;
    }

    let env = provider.to_uppercase();
    messages.push(format!(
        "Register an OAuth app with {provider} whose callback URL is \
         <your app URL>/auth/{provider}/callback, then set {env}_CLIENT_ID and \
         {env}_CLIENT_SECRET (and SESSION_SECRET in production)."
    ));
    results.rrgen.extend(
        messages
            .into_iter()
            .map(|message| crate::GenResult::Generated {
                message: Some(message),
            }),
    );
    Ok(results)
}

fn validate_name(provider: &str) -> Result<()> {
    let ok = !provider.is_empty()
        && provider
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && provider != "sign_out";
    if ok {
        Ok(())
    } else {
        Err(Error::Message(format!(
            "`{provider}` is not a usable provider name: use lowercase letters, digits and `_` \
             (e.g. github, google, okta)"
        )))
    }
}

/// The provider's settings. GitHub and Google are presets; anything else is a
/// generic OAuth2/OIDC provider whose endpoints the user fills in.
fn provider_stanza(provider: &str, env: &str) -> Vec<String> {
    let var = provider.to_uppercase();
    let value = |name: &str| match env {
        "test" => format!("test-{}", name.to_lowercase().replace('_', "-")),
        "production" => format!("<%= get_env(name=\"{var}_{name}\") %>"),
        _ => format!("<%= get_env(name=\"{var}_{name}\", default=\"\") %>"),
    };
    let mut lines = vec![
        format!("{provider}:"),
        format!("  client_id: {}", value("CLIENT_ID")),
        format!("  client_secret: {}", value("CLIENT_SECRET")),
    ];
    if !matches!(provider, "github" | "google") {
        lines.extend([
            "  strategy: generic".to_string(),
            "  authorize_url: https://id.example.com/oauth2/authorize".to_string(),
            "  token_url: https://id.example.com/oauth2/token".to_string(),
            "  userinfo_url: https://id.example.com/oauth2/userinfo".to_string(),
        ]);
    }
    lines
}

fn session_stanza(env: &str) -> Vec<String> {
    let secret = match env {
        "production" => "<%= get_env(name=\"SESSION_SECRET\") %>".to_string(),
        // Not a secret: development and test cookies need only be stable.
        _ => format!("{env}-session-secret-change-me-0123456789abcdef"),
    };
    vec![
        "session:".to_string(),
        "  # Signs the session cookie. Production reads it from SESSION_SECRET;".to_string(),
        "  # generate one with `openssl rand -base64 64`.".to_string(),
        format!("  secret: {secret}"),
    ]
}

fn indent(lines: &[String], by: usize) -> Vec<String> {
    let pad = " ".repeat(by);
    lines.iter().map(|l| format!("{pad}{l}")).collect()
}

/// Whether `line` opens a top-level key (or is the end of the file's
/// indented block).
fn is_top_level(line: &str) -> bool {
    !line.is_empty() && !line.starts_with(' ') && !line.starts_with('#')
}

/// Finds a key exactly `indent` spaces deep inside `lines[from..to]`.
fn find_key(lines: &[String], from: usize, to: usize, indent: usize, key: &str) -> Option<usize> {
    let wanted = format!("{}{key}:", " ".repeat(indent));
    (from..to).find(|&i| {
        lines[i] == wanted
            || lines[i]
                .strip_prefix(&wanted)
                .is_some_and(|rest| rest.starts_with([' ', '#']))
    })
}

/// End (exclusive) of the block opened at `start`, whose children are indented
/// deeper than `indent`.
fn block_end(lines: &[String], start: usize, indent: usize) -> usize {
    (start + 1..lines.len())
        .find(|&i| {
            let line = &lines[i];
            let depth = line.len() - line.trim_start().len();
            !line.trim().is_empty() && !line.trim_start().starts_with('#') && depth <= indent
        })
        .unwrap_or(lines.len())
}

/// Adds `provider` (and, if absent, a session secret) under `auth:` in a Loco
/// config file. Works on text, not parsed YAML, so comments and `<%= %>`
/// expressions survive; it expects the two-space indentation Loco generates.
///
/// # Errors
///
/// When the provider is already configured.
pub fn add_provider(
    content: &str,
    provider: &str,
    env: &str,
) -> std::result::Result<String, String> {
    let mut lines: Vec<String> = content.lines().map(str::to_string).collect();

    let Some(auth) =
        (0..lines.len()).find(|&i| lines[i] == "auth:" || lines[i].starts_with("auth: "))
    else {
        let mut block = vec![
            String::new(),
            "# Authentication Configuration".to_string(),
            "auth:".to_string(),
        ];
        block.extend(indent(&session_stanza(env), 2));
        block.push("  # OAuth sign-in (`cargo loco generate oauth <provider>`)".to_string());
        block.push("  oauth:".to_string());
        block.push("    providers:".to_string());
        block.extend(indent(&provider_stanza(provider, env), 6));
        lines.extend(block);
        return Ok(finish(&lines, content));
    };
    let auth_end = (auth + 1..lines.len())
        .find(|&i| is_top_level(&lines[i]))
        .unwrap_or(lines.len());

    let mut inserts: Vec<(usize, Vec<String>)> = vec![];
    if let Some(oauth) = find_key(&lines, auth + 1, auth_end, 2, "oauth") {
        let oauth_end = block_end(&lines, oauth, 2);
        if let Some(providers) = find_key(&lines, oauth + 1, oauth_end, 4, "providers") {
            let providers_end = block_end(&lines, providers, 4);
            if find_key(&lines, providers + 1, providers_end, 6, provider).is_some() {
                return Err(format!("auth.oauth.providers.{provider} already exists"));
            }
            inserts.push((providers_end, indent(&provider_stanza(provider, env), 6)));
        } else {
            let mut block = vec!["    providers:".to_string()];
            block.extend(indent(&provider_stanza(provider, env), 6));
            inserts.push((oauth_end, block));
        }
    } else {
        let mut block =
            vec!["  # OAuth sign-in (`cargo loco generate oauth <provider>`)".to_string()];
        block.push("  oauth:".to_string());
        block.push("    providers:".to_string());
        block.extend(indent(&provider_stanza(provider, env), 6));
        inserts.push((block_end(&lines, auth, 0), block));
    }
    if find_key(&lines, auth + 1, auth_end, 2, "session").is_none() {
        inserts.push((auth + 1, indent(&session_stanza(env), 2)));
    }
    // Back to front, so earlier indices stay valid.
    inserts.sort_by_key(|a| std::cmp::Reverse(a.0));
    for (at, block) in inserts {
        // Keep blank lines that separate the block from the next section
        // after the insertion.
        let mut at = at;
        while at > auth + 1 && lines[at - 1].trim().is_empty() {
            at -= 1;
        }
        lines.splice(at..at, block);
    }
    Ok(finish(&lines, content))
}

fn finish(lines: &[String], original: &str) -> String {
    let mut out = lines.join("\n");
    if original.ends_with('\n') || original.is_empty() {
        out.push('\n');
    }
    out
}

/// Outcome of [`enable_feature`].
#[derive(Debug, PartialEq, Eq)]
pub enum Edit {
    /// The new file contents.
    Changed(String),
    /// Already enabled.
    Unchanged,
}

/// Adds `feature` to the `loco-rs` dependency in `[dependencies]`, or `None`
/// when the dependency is not written in a shape this can safely edit (a
/// multi-line table, say).
#[must_use]
pub fn enable_feature(cargo_toml: &str, feature: &str) -> Option<Edit> {
    let mut lines: Vec<String> = cargo_toml.lines().map(str::to_string).collect();
    let deps = lines.iter().position(|l| l.trim() == "[dependencies]")?;
    let end = (deps + 1..lines.len())
        .find(|&i| lines[i].trim_start().starts_with('['))
        .unwrap_or(lines.len());
    let i = (deps + 1..end).find(|&i| {
        let l = lines[i].trim_start();
        l.starts_with("loco-rs ") || l.starts_with("loco-rs=")
    })?;
    let line = lines[i].clone();
    let quoted = format!("\"{feature}\"");
    if line.contains(&quoted) {
        return Some(Edit::Unchanged);
    }
    let updated = if let Some(pos) = line.find("features = [") {
        let at = pos + "features = [".len();
        let rest = &line[at..];
        let sep = if rest.trim_start().starts_with(']') {
            ""
        } else {
            ", "
        };
        format!("{}{quoted}{sep}{rest}", &line[..at])
    } else if let Some(stripped) = line.trim_end().strip_suffix('}') {
        format!("{}, features = [{quoted}] }}", stripped.trim_end())
    } else if let Some((name, version)) = line.split_once('=') {
        let version = version.trim();
        version
            .starts_with('"')
            .then(|| format!("{name}= {{ version = {version}, features = [{quoted}] }}"))?
    } else {
        return None;
    };
    lines[i] = updated;
    Some(Edit::Changed(finish(&lines, cargo_toml)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_is_added_to_every_dependency_shape() {
        let cases = [
            (
                "[dependencies]\nloco-rs = { workspace = true }\n",
                "[dependencies]\nloco-rs = { workspace = true, features = [\"auth_oauth\"] }\n",
            ),
            (
                "[dependencies]\nloco-rs = { workspace = true, features = [\"with-db\"] }\n",
                "[dependencies]\nloco-rs = { workspace = true, features = [\"auth_oauth\", \"with-db\"] }\n",
            ),
            (
                "[dependencies]\nloco-rs = { workspace = true, features = [] }\n",
                "[dependencies]\nloco-rs = { workspace = true, features = [\"auth_oauth\"] }\n",
            ),
            (
                "[dependencies]\nloco-rs = \"1.2\"\n",
                "[dependencies]\nloco-rs = { version = \"1.2\", features = [\"auth_oauth\"] }\n",
            ),
        ];
        for (before, after) in cases {
            assert_eq!(
                enable_feature(before, "auth_oauth"),
                Some(Edit::Changed(after.to_string())),
                "{before}"
            );
        }
    }

    #[test]
    fn only_dependencies_are_edited() {
        let toml = "[workspace.dependencies]\nloco-rs = { version = \"1\" }\n\n[dependencies]\n\
                    loco-rs = { workspace = true }\n\n[dev-dependencies]\nloco-rs = { workspace = true, features = [\"testing\"] }\n";
        let Some(Edit::Changed(out)) = enable_feature(toml, "auth_oauth") else {
            panic!("expected an edit");
        };
        assert!(out.contains("[workspace.dependencies]\nloco-rs = { version = \"1\" }"));
        assert!(out.contains(
            "[dependencies]\nloco-rs = { workspace = true, features = [\"auth_oauth\"] }"
        ));
        assert!(out.contains("features = [\"testing\"] }"));
        assert_eq!(enable_feature(&out, "auth_oauth"), Some(Edit::Unchanged));
        assert_eq!(
            enable_feature("[package]\nname = \"x\"\n", "auth_oauth"),
            None
        );
    }

    #[test]
    fn provider_names_are_validated() {
        assert!(validate_name("github").is_ok());
        assert!(validate_name("my_idp2").is_ok());
        for bad in ["", "GitHub", "a-b", "a/b", "sign_out"] {
            assert!(validate_name(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_second_provider_joins_the_first() {
        let once = add_provider("server:\n  port: 5150\n", "github", "development").unwrap();
        let twice = add_provider(&once, "google", "development").unwrap();
        assert_eq!(twice.matches("    providers:").count(), 1, "{twice}");
        assert_eq!(twice.matches("  session:").count(), 1, "{twice}");
        let github = twice.find("      github:").unwrap();
        let google = twice.find("      google:").unwrap();
        assert!(github < google);
        assert!(add_provider(&twice, "github", "development")
            .unwrap_err()
            .contains("already exists"));
    }
}
