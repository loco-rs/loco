//! This module provides interactive utilities for setting up application
//! configurations based on user input.

use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use strum::IntoEnumIterator;

use crate::{
    wizard_opts::{self, AssetsOption, BackgroundOption, DBOption},
    Error,
};

/// Holds the user's configuration selections.
pub struct Selections {
    pub db: wizard_opts::DBOption,
    pub background: wizard_opts::BackgroundOption,
    pub asset: wizard_opts::AssetsOption,
}

impl Selections {
    #[must_use]
    pub fn message(&self) -> Vec<String> {
        let mut res = Vec::new();
        if let Some(m) = self.db.user_message() {
            res.push(m);
        }
        if let Some(m) = self.background.user_message() {
            res.push(m);
        }
        if let Some(m) = self.asset.user_message() {
            res.push(m);
        }
        res
    }
}

/// Prompts the user to enter an application name, with optional pre-set name
/// input. Validates the name to ensure compliance with required naming rules.
/// Returns the validated name or an error if validation fails.
///
/// # Errors
/// when could not show user selection
pub fn app_name(name: Option<String>) -> crate::Result<String> {
    if let Some(app_name) = name {
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

/// Warns the user if the current directory is inside a Git repository and
/// prompts them to confirm whether they wish to proceed. If declined, an error
/// is returned.
///
/// # Errors
/// when could not show user selection or user chose not continue
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

/// Validates the application name.
fn validate_app_name(app_name: &str) -> Result<(), &str> {
    if app_name.is_empty() {
        return Err("app name could not be empty");
    }

    let mut chars = app_name.chars();
    if let Some(ch) = chars.next() {
        if ch.is_ascii_digit() {
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

/// Provides a selection menu to the user for choosing from a list of options.
/// Returns the selected option or a default if selection fails.
fn select_option<T>(text: &str, options: &[T]) -> crate::Result<T>
where
    T: Default + ToString + Clone,
{
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(text)
        .default(0)
        .items(options)
        .interact()?;
    Ok(options.get(selection).cloned().unwrap_or_default())
}

/// start wizard
///
/// # Errors
/// when could not show user selection or user chose not continue
pub fn start(args: &wizard_opts::ArgsPlaceholder) -> crate::Result<Selections> {
    let template = select_option(
        "❯ What would you like to build?",
        &wizard_opts::Template::iter().collect::<Vec<_>>(),
    )?;

    match template {
        wizard_opts::Template::Lightweight => Ok(Selections {
            db: DBOption::None,
            background: BackgroundOption::None,
            asset: AssetsOption::None,
        }),
        wizard_opts::Template::RestApi => Ok(Selections {
            db: select_db(args)?,
            background: select_background(args)?,
            asset: AssetsOption::None,
        }),
        wizard_opts::Template::SaasServerSideRendering => Ok(Selections {
            db: select_db(args)?,
            background: select_background(args)?,
            asset: AssetsOption::Serverside,
        }),
        wizard_opts::Template::SaasClientSideRendering => Ok(Selections {
            db: select_db(args)?,
            background: select_background(args)?,
            asset: AssetsOption::Clientside,
        }),
        wizard_opts::Template::Advanced => Ok(Selections {
            db: select_db(args)?,
            background: select_background(args)?,
            asset: select_asset(args)?,
        }),
    }
}

/// Prompts the user to select a database option if none is provided in the
/// arguments.
fn select_db(args: &wizard_opts::ArgsPlaceholder) -> crate::Result<DBOption> {
    let dboption = if let Some(dboption) = args.db.clone() {
        dboption
    } else {
        select_option(
            "❯ Select a DB Provider",
            &wizard_opts::DBOption::iter().collect::<Vec<_>>(),
        )?
    };
    Ok(dboption)
}

/// Prompts the user to select a background worker option if none is provided in
/// the arguments.
fn select_background(args: &wizard_opts::ArgsPlaceholder) -> crate::Result<BackgroundOption> {
    let bgopt = if let Some(bgopt) = args.bg.clone() {
        bgopt
    } else {
        select_option(
            "❯ Select your background worker type",
            &wizard_opts::BackgroundOption::iter().collect::<Vec<_>>(),
        )?
    };
    Ok(bgopt)
}

/// Prompts the user to select an asset configuration if none is provided in the
/// arguments.
fn select_asset(args: &wizard_opts::ArgsPlaceholder) -> crate::Result<AssetsOption> {
    let assetopt = if let Some(assetopt) = args.assets.clone() {
        assetopt
    } else {
        select_option(
            "❯ Select an asset serving configuration",
            &wizard_opts::AssetsOption::iter().collect::<Vec<_>>(),
        )?
    };
    Ok(assetopt)
}
