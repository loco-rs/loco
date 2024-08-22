use std::{collections::BTreeMap, env};

use dialoguer::{theme::ColorfulTheme, Select};
use lazy_static::lazy_static;
use regex::Regex;
use strum::IntoEnumIterator;

use crate::{
    env_vars,
    generate::{self, AssetsOption, BackgroundOption, DBOption, OptionsList},
};

lazy_static! {
    static ref VALIDATE_APP_NAME: Regex = Regex::new(r"^[a-zA-Z0-9_]+$").unwrap();
}

/// Prompts the user to enter a valid application name for use with the Loco app
/// generator.
///
/// If the `LOCO_APP_NAME` environment variable is set, the function attempts to
/// use the specified app name directly. If the environment variable is not set
/// or the specified app name is invalid, the function prompts the user to enter
/// a valid app name interactively.
///
/// # Errors
/// when could not prompt the question to the user or enter value is empty
pub fn app_name() -> eyre::Result<String> {
    if let Ok(app_name) = env::var(env_vars::APP_NAME) {
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
/// If the `LOCO_TEMPLATE` environment variable is set, the function attempts to
/// use the template with the specified name directly. If the environment
/// variable is not set or the specified template is not found, the function
/// presents a list of available templates to the user for selection.
///
/// # Errors
/// when could not prompt the question to the user or enter value is empty
pub fn template_selection(
    templates: &BTreeMap<String, generate::Template>,
) -> eyre::Result<(String, generate::Template)> {
    if let Ok(template_name) = env::var(env_vars::TEMPLATE) {
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

/// Warn the user if they are inside a git repository.
///
/// If the `ALLOW_IN_GIT_REPO` environment variable is set, this test will be
/// skipped. If the environment variable is not set, the function will warn the
/// user if they are inside a git and will let them cancel the operation or
/// continue.
///
/// # Errors
/// when could not prompt the question, or when the user choose not to continue.
pub fn warn_if_in_git_repo() -> eyre::Result<()> {
    let question = requestty::Question::confirm("allow_git_repo")
        .message("❯ You are inside a git repository. Do you wish to continue?")
        .default(false)
        .build();

    let res = requestty::prompt_one(question)?;
    let answer = res
        .as_bool()
        .ok_or_else(|| eyre::eyre!("allow_git_repo is empty"))?;

    if answer {
        Ok(())
    } else {
        Err(eyre::eyre!("Aborted: You've chose not to continue."))
    }
}

/// Validates the provided application name for compatibility with Rust library
/// conventions.
///
/// Rust library names should adhere to specific conventions and avoid special
/// characters to ensure compatibility with Cargo, the Rust package manager.
/// This function checks whether the provided application name complies with
/// these conventions.
fn validate_app_name(app: &str) -> Result<(), String> {
    if !VALIDATE_APP_NAME.is_match(app) {
        return Err(
            "app name is invalid, illegal characters. keep names simple: myapp or my_app"
                .to_owned(),
        );
    }

    Ok(())
}

fn select_option<T>(
    text: &str,
    kind: &OptionsList,
    options: &[T],
    optionsmap: &[OptionsList],
) -> crate::Result<T>
where
    T: Default + ToString + Clone,
{
    let opt = if optionsmap.contains(kind) {
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(text)
            .default(0)
            .items(options)
            .interact()?;
        options.get(selection).cloned().unwrap_or_default()
    } else {
        T::default()
    };
    Ok(opt)
}

pub(crate) fn options_selection(
    template: &generate::Template,
) -> crate::Result<(DBOption, BackgroundOption, AssetsOption)> {
    let optionsmap = template.options.clone().unwrap_or_default();
    let dboption = select_option(
        "pick db",
        &OptionsList::DB,
        &DBOption::iter().collect::<Vec<_>>(),
        &optionsmap,
    )?;
    let bgopt = select_option(
        "pick background",
        &OptionsList::Background,
        &BackgroundOption::iter().collect::<Vec<_>>(),
        &optionsmap,
    )?;
    let assetopt = select_option(
        "pick asset",
        &OptionsList::Assets,
        &AssetsOption::iter().collect::<Vec<_>>(),
        &optionsmap,
    )?;
    Ok((dboption, bgopt, assetopt))
}
