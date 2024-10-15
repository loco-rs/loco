use std::{collections::BTreeMap, env};

use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use strum::IntoEnumIterator;

use crate::{
    env_vars,
    generate::{self, ArgsPlaceholder, AssetsOption, BackgroundOption, DBOption, OptionsList},
    Error,
};

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
pub fn app_name(name: Option<String>) -> crate::Result<String> {
    if let Some(app_name) = env::var(env_vars::APP_NAME).ok().or(name) {
        validate_app_name(app_name.as_str()).map_err(|err| Error::msg(err.to_string()))?;
        Ok(app_name)
    } else {
        let res = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("❯ App name?")
            .default("myapp".into())
            .validate_with(|input: &String| {
                if let Err(err) = validate_app_name(input) {
                    Err(err.to_string())
                } else {
                    Ok(())
                }
            })
            .interact_text()?;
        Ok(res)
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
    args: &generate::ArgsPlaceholder,
) -> crate::Result<(String, generate::Template)> {
    if let Some(template_name) = env::var(env_vars::TEMPLATE)
        .ok()
        .or_else(|| args.template.clone())
    {
        templates.get(&template_name).map_or_else(
            || Err(Error::msg(format!("no such template: `{template_name}`"))),
            |template| Ok((template_name.to_string(), template.clone())),
        )
    } else {
        let options: Vec<String> = templates
            .iter()
            .map(|t| t.1.description.to_string())
            .collect();

        let selection_index = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("❯ What would you like to build?")
            .items(&options)
            .default(0)
            .interact()?;

        let selection = options
            .get(selection_index)
            .ok_or_else(|| Error::msg("template selection is empty".to_string()))?;

        for (name, template) in templates {
            if template.description == *selection {
                return Ok((name.to_string(), template.clone()));
            }
        }
        Err(Error::msg("selection invalid".to_string()))
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
pub fn warn_if_in_git_repo() -> crate::Result<()> {
    let answer = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("❯ You are inside a git repository. Do you wish to continue?")
        .default(false)
        .interact()?;

    if answer {
        Ok(())
    } else {
        Err(Error::msg(
            "Aborted: You've chose not to continue.".to_string(),
        ))
    }
}

/// Validates the provided application name for compatibility with Rust library
/// conventions.
///
/// Rust library names should adhere to specific conventions and avoid special
/// characters to ensure compatibility with Cargo, the Rust package manager.
/// This function checks whether the provided application name complies with
/// these conventions.
fn validate_app_name(app_name: &str) -> Result<(), &str> {
    if app_name.is_empty() {
        return Err("app name could not be empty");
    }

    let mut chars = app_name.chars();
    if let Some(ch) = chars.next() {
        if ch.is_digit(10) {
            return Err("the name cannot start with a digit");
        }
        if !(unicode_xid::UnicodeXID::is_xid_start(ch) || ch == '_') {
            return Err(
                "the first character must be a Unicode XID start character (most letters or `_`)",
            );
        }
    }
    for ch in chars {
        if !(unicode_xid::UnicodeXID::is_xid_continue(ch) || ch == '-') {
            return Err(
                "characters must be Unicode XID characters (numbers, `-`, `_`, or most letters)",
            );
        }
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
    args: &ArgsPlaceholder,
) -> crate::Result<(DBOption, BackgroundOption, AssetsOption)> {
    let optionsmap = template.options.clone().unwrap_or_default();
    let dboption = if let Some(dboption) = args.db.clone() {
        dboption
    } else {
        select_option(
            "❯ Select a DB Provider",
            &OptionsList::DB,
            &DBOption::iter().collect::<Vec<_>>(),
            &optionsmap,
        )?
    };

    let bgopt = if let Some(bgopt) = args.bg.clone() {
        bgopt
    } else {
        select_option(
            "❯ Select your background worker type",
            &OptionsList::Background,
            &BackgroundOption::iter().collect::<Vec<_>>(),
            &optionsmap,
        )?
    };

    let assetopt = if let Some(assetopt) = args.assets.clone() {
        assetopt
    } else {
        select_option(
            "❯ Select an asset serving configuration",
            &OptionsList::Assets,
            &AssetsOption::iter().collect::<Vec<_>>(),
            &optionsmap,
        )?
    };

    Ok((dboption, bgopt, assetopt))
}
