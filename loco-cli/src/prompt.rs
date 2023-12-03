use crate::generate;
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::BTreeMap;
use std::env;

lazy_static! {
    static ref VALIDATE_APP_NAME: Regex = Regex::new(r"^[a-zA-Z0-9_]+$").unwrap();
}

/// Prompts the user to enter a valid application name for use with the Loco app generator.
///
/// If the `LOGO_APP_NAME` environment variable is set, the function attempts to use the specified
/// app name directly. If the environment variable is not set or the specified app name is invalid, the
/// function prompts the user to enter a valid app name interactively.
///
/// # Errors
/// when could not prompt the question to the user or enter value is empty
pub fn app_name() -> eyre::Result<String> {
    if let Ok(app_name) = env::var("LOGO_APP_NAME") {
        validate_app_name(app_name.as_str()).map_err(|e| eyre::eyre!(e))?;
        Ok(app_name)
    } else {
        let app_name = requestty::Question::input("app_name")
            .message("❯ App name?")
            .default("myapp")
            .validate(|ans, _| validate_app_name(ans))
            .build();

        let res = requestty::prompt_one(app_name)?;
        Ok(res
            .as_string()
            .ok_or_else(|| eyre::eyre!("app selection name is empty"))?
            .to_string())
    }
}

/// Prompts the user to select a template from the provided template list.
///
/// If the `LOCO_TEMPLATE` environment variable is set, the function attempts to use the template with
/// the specified name directly. If the environment variable is not set or the specified template is not
/// found, the function presents a list of available templates to the user for selection.
///
/// # Errors
/// when could not prompt the question to the user or enter value is empty
///
pub fn template_selection(
    templates: &BTreeMap<String, generate::Template>,
) -> eyre::Result<(String, generate::Template)> {
    if let Ok(template_name) = env::var("LOCO_TEMPLATE") {
        templates.get(&template_name).map_or_else(
            || Err(eyre::eyre!("template env var is invalid")),
            |template| Ok((template_name.to_string(), template.clone())),
        )
    } else {
        let options: Vec<String> = templates
            .iter()
            .map(|t| t.1.description.to_string())
            .collect();

        let order_select = requestty::Question::select("template")
            .message("❯ What would you like to build?")
            .choices(&options)
            .build();

        let answer = requestty::prompt_one(order_select)?;

        let selection = answer
            .as_list_item()
            .ok_or_else(|| eyre::eyre!("template selection it empty"))?;

        for (name, template) in templates {
            if template.description == selection.text {
                return Ok((name.to_string(), template.clone()));
            }
        }
        Err(eyre::eyre!("selection invalid"))
    }
}

/// Validates the provided application name for compatibility with Rust library conventions.
///
/// Rust library names should adhere to specific conventions and avoid special characters to
/// ensure compatibility with Cargo, the Rust package manager. This function checks whether the
/// provided application name complies with these conventions.
fn validate_app_name(app: &str) -> Result<(), String> {
    if !VALIDATE_APP_NAME.is_match(app) {
        return Err(
            "app name is invalid, illegal characters. keep names simple: myapp or my_app"
                .to_owned(),
        );
    }

    Ok(())
}
