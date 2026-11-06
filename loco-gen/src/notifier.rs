//! `cargo loco generate notifier <name>`: a notification type for an app.
//!
//! Renders `src/notifiers/<name>.rs`, then does what a template cannot: create
//! `src/notifiers/mod.rs` on first use, add a default `notifications:` block
//! to each `config/*.yaml` that has none, and enable the `notifications`
//! feature on `loco-rs` in `Cargo.toml`.

use std::{fs, path::Path};

use heck::ToSnakeCase;
use serde_json::json;

use super::{AppInfo, GenerateResults, Result};
use crate::{
    oauth::{enable_feature, Edit},
    RRgen,
};

const ENVIRONMENTS: [&str; 3] = ["development", "test", "production"];

/// Generates a notifier named `name`.
///
/// # Errors
///
/// When a file cannot be read or written, or the template fails to render.
pub fn generate(rrgen: &RRgen, name: &str, appinfo: &AppInfo) -> Result<GenerateResults> {
    let root = &appinfo.working_dir;

    // The template injects into `mod.rs`, so it must exist first. Created
    // here and removed again if rendering fails, so a failed run is a no-op.
    let mod_rs = root.join("src").join("notifiers").join("mod.rs");
    let created_mod = !mod_rs.exists();
    if created_mod {
        if let Some(dir) = mod_rs.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(
            &mod_rs,
            "//! Notifiers: `cargo loco generate notifier <name>`.\n",
        )?;
    }
    let mut results = match crate::render_template(
        rrgen,
        Path::new("notifier"),
        &json!({"name": name.to_snake_case(), "pkg_name": appinfo.app_name}),
    ) {
        Ok(results) => results,
        Err(err) => {
            if created_mod {
                let _ = fs::remove_file(&mod_rs);
            }
            return Err(err);
        }
    };

    let mut messages = vec![];
    for env in ENVIRONMENTS {
        let path = root.join("config").join(format!("{env}.yaml"));
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        if let Some(updated) = add_config(&content, env) {
            fs::write(&path, updated)?;
        }
    }

    let cargo_toml = root.join("Cargo.toml");
    match fs::read_to_string(&cargo_toml)
        .ok()
        .and_then(|c| enable_feature(&c, "notifications"))
    {
        Some(Edit::Changed(updated)) => fs::write(&cargo_toml, updated)?,
        Some(Edit::Unchanged) => {}
        None => messages.push(
            "Enable the `notifications` feature on `loco-rs` in Cargo.toml: \
             loco-rs = { ..., features = [\"notifications\"] }"
                .to_string(),
        ),
    }
    messages.push(
        "Set SLACK_WEBHOOK_URL to a Slack incoming-webhook URL, or edit `notifications:` in \
         config/*.yaml. Unconfigured channels are skipped."
            .to_string(),
    );

    results.rrgen.extend(
        messages
            .into_iter()
            .map(|message| crate::GenResult::Generated {
                message: Some(message),
            }),
    );
    Ok(results)
}

/// Appends a `notifications:` block, unless the file already has one.
fn add_config(content: &str, env: &str) -> Option<String> {
    if content
        .lines()
        .any(|l| l.trim_end() == "notifications:" || l.starts_with("notifications:"))
    {
        return None;
    }
    let block = if env == "test" {
        // Tests record deliveries (`notifications::testing::deliveries`)
        // instead of sending them, like the mailer's `stub: true`.
        "notifications:\n  stub: true\n  slack:\n    webhook_url: \"https://hooks.slack.com/services/test\"\n"
            .to_string()
    } else {
        "notifications:\n  slack:\n    webhook_url: <%= get_env(name=\"SLACK_WEBHOOK_URL\", default=\"\") %>\n"
            .to_string()
    };
    let mut out = content.to_string();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(
        "\n# Notifier channels (`cargo loco generate notifier`). A channel whose URL is\n\
         # empty is skipped.\n",
    );
    out.push_str(&block);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_block_is_added_once() {
        let added = add_config("logger:\n  enable: true\n", "development").unwrap();
        assert!(added.contains("notifications:\n  slack:\n    webhook_url: <%= get_env("));
        assert_eq!(add_config(&added, "development"), None);
    }

    #[test]
    fn test_env_records_instead_of_sending() {
        let added = add_config("", "test").unwrap();
        assert!(added.contains("notifications:\n  stub: true\n"), "{added}");
        assert!(!added.contains("get_env"), "{added}");
    }
}
