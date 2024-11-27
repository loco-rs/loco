//! This module provides interactive utilities for setting up application
//! configurations based on user input.

use clap::ValueEnum;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, IntoEnumIterator};

use crate::Error;

#[derive(
    Debug, Clone, Deserialize, Serialize, EnumIter, Display, Default, PartialEq, Eq, ValueEnum,
)]
pub enum Template {
    #[default]
    #[strum(to_string = "Saas App with server side rendering")]
    SaasServerSideRendering,
    #[strum(to_string = "Saas App with client side rendering")]
    SaasClientSideRendering,
    #[strum(to_string = "Rest API (with DB and user auth)")]
    RestApi,
    #[strum(to_string = "lightweight-service (minimal, only controllers and views)")]
    Lightweight,
    #[strum(to_string = "Advanced")]
    Advanced,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum OptionsList {
    #[serde(rename = "db")]
    DB,
    #[serde(rename = "bg")]
    Background,
    #[serde(rename = "assets")]
    Assets,
}

#[derive(
    Debug, Clone, Deserialize, Serialize, EnumIter, Display, Default, PartialEq, Eq, ValueEnum,
)]
pub enum DBOption {
    #[default]
    #[serde(rename = "sqlite")]
    Sqlite,
    #[serde(rename = "pg")]
    Postgres,
    #[serde(rename = "none")]
    None,
}

impl DBOption {
    #[must_use]
    pub const fn enable(&self) -> bool {
        !matches!(self, Self::None)
    }

    #[must_use]
    pub fn user_message(&self) -> Option<String> {
        match self {
            Self::Postgres => Some(format!(
                "{}: You've selected `{}` as your DB provider (you should have a postgres \
                 instance to connect to)",
                "database".underline(),
                "postgres".yellow()
            )),
            Self::Sqlite | Self::None => None,
        }
    }

    #[must_use]
    pub const fn endpoint_config(&self) -> &str {
        match self {
            Self::Sqlite => "sqlite://NAME_ENV.sqlite?mode=rwc",
            Self::Postgres => "postgres://loco:loco@localhost:5432/NAME_ENV",
            Self::None => "",
        }
    }
}

#[derive(
    Debug, Clone, Deserialize, Serialize, EnumIter, Display, Default, PartialEq, Eq, ValueEnum,
)]
pub enum BackgroundOption {
    #[default]
    #[strum(to_string = "Async (in-process tokio async tasks)")]
    #[serde(rename = "BackgroundAsync")]
    Async,
    #[strum(to_string = "Queue (standalone workers using Redis)")]
    #[serde(rename = "BackgroundQueue")]
    Queue,
    #[strum(to_string = "Blocking (run tasks in foreground)")]
    #[serde(rename = "ForegroundBlocking")]
    Blocking,
    #[strum(to_string = "None")]
    #[serde(rename = "none")]
    None,
}

impl BackgroundOption {
    #[must_use]
    pub const fn enable(&self) -> bool {
        !matches!(self, Self::None)
    }

    #[must_use]
    pub fn user_message(&self) -> Option<String> {
        match self {
            Self::Queue => Some(format!(
                "{}: You've selected `{}` for your background worker configuration (you should \
                 have a Redis/valkey instance to connect to)",
                "workers".underline(),
                "queue".yellow()
            )),
            Self::Blocking => Some(format!(
                "{}: You've selected `{}` for your background worker configuration. Your workers \
                 configuration will BLOCK REQUESTS until a task is done.",
                "workers".underline(),
                "blocking".yellow()
            )),
            Self::Async | Self::None => None,
        }
    }

    #[must_use]
    pub const fn prompt_view(&self) -> &str {
        match self {
            Self::Async => "Async",
            Self::Queue => "BackgroundQueue",
            Self::Blocking => "ForegroundBlocking",
            Self::None => "None",
        }
    }
}

#[derive(
    Debug, Clone, Deserialize, Serialize, EnumIter, Display, Default, PartialEq, Eq, ValueEnum,
)]
pub enum AssetsOption {
    #[default]
    #[strum(to_string = "Server (configures server-rendered views)")]
    #[serde(rename = "server")]
    Serverside,
    #[strum(to_string = "Client (configures assets for frontend serving)")]
    #[serde(rename = "client")]
    Clientside,
    #[strum(to_string = "None")]
    #[serde(rename = "none")]
    None,
}

impl AssetsOption {
    #[must_use]
    pub const fn enable(&self) -> bool {
        !matches!(self, Self::None)
    }

    #[must_use]
    pub fn user_message(&self) -> Option<String> {
        match self {
            Self::Clientside => Some(format!(
                "{}: You've selected `{}` for your asset serving configuration.\n\nNext step, \
                 build your frontend:\n  $ cd {}\n  $ npm install && npm run build\n",
                "assets".underline(),
                "clientside".yellow(),
                "frontend/".yellow()
            )),
            Self::Serverside | Self::None => None,
        }
    }
}

#[derive(Debug, Clone, Default)]
/// Represents internal placeholders to be replaced.
pub struct ArgsPlaceholder {
    pub db: Option<DBOption>,
    pub bg: Option<BackgroundOption>,
    pub assets: Option<AssetsOption>,
}

/// Holds the user's configuration selections.
pub struct Selections {
    pub db: DBOption,
    pub background: BackgroundOption,
    pub asset: AssetsOption,
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
pub fn start(args: &ArgsPlaceholder) -> crate::Result<Selections> {
    // user provided everything via flags so no need to prompt, just return
    if let (Some(db), Some(bg), Some(assets)) =
        (args.db.clone(), args.bg.clone(), args.assets.clone())
    {
        return Ok(Selections {
            db,
            background: bg,
            asset: assets,
        });
    }

    let template = select_option(
        "❯ What would you like to build?",
        &Template::iter().collect::<Vec<_>>(),
    )?;

    match template {
        Template::Lightweight => Ok(Selections {
            db: DBOption::None,
            background: BackgroundOption::None,
            asset: AssetsOption::None,
        }),
        Template::RestApi => Ok(Selections {
            db: select_db(args)?,
            background: select_background(args)?,
            asset: AssetsOption::None,
        }),
        Template::SaasServerSideRendering => Ok(Selections {
            db: select_db(args)?,
            background: select_background(args)?,
            asset: AssetsOption::Serverside,
        }),
        Template::SaasClientSideRendering => Ok(Selections {
            db: select_db(args)?,
            background: select_background(args)?,
            asset: AssetsOption::Clientside,
        }),
        Template::Advanced => Ok(Selections {
            db: select_db(args)?,
            background: select_background(args)?,
            asset: select_asset(args)?,
        }),
    }
}

/// Prompts the user to select a database option if none is provided in the
/// arguments.
fn select_db(args: &ArgsPlaceholder) -> crate::Result<DBOption> {
    let dboption = if let Some(dboption) = args.db.clone() {
        dboption
    } else {
        select_option(
            "❯ Select a DB Provider",
            &DBOption::iter().collect::<Vec<_>>(),
        )?
    };
    Ok(dboption)
}

/// Prompts the user to select a background worker option if none is provided in
/// the arguments.
fn select_background(args: &ArgsPlaceholder) -> crate::Result<BackgroundOption> {
    let bgopt = if let Some(bgopt) = args.bg.clone() {
        bgopt
    } else {
        select_option(
            "❯ Select your background worker type",
            &BackgroundOption::iter().collect::<Vec<_>>(),
        )?
    };
    Ok(bgopt)
}

/// Prompts the user to select an asset configuration if none is provided in the
/// arguments.
fn select_asset(args: &ArgsPlaceholder) -> crate::Result<AssetsOption> {
    let assetopt = if let Some(assetopt) = args.assets.clone() {
        assetopt
    } else {
        select_option(
            "❯ Select an asset serving configuration",
            &AssetsOption::iter().collect::<Vec<_>>(),
        )?
    };
    Ok(assetopt)
}
