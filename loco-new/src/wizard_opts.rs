use clap::ValueEnum;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};

#[derive(
    Debug, Clone, Deserialize, Serialize, EnumIter, Display, Default, PartialEq, Eq, ValueEnum,
)]
pub enum Template {
    #[default]
    #[strum(to_string = "lightweight-service (minimal, only controllers and views)")]
    Lightweight,
    #[strum(to_string = "Rest API (with DB and user auth)")]
    RestApi,
    #[strum(to_string = "Saas App with server side rendering")]
    SaasServerSideRendering,
    #[strum(to_string = "Saas App with client side rendering")]
    SaasClientSideRendering,
    #[strum(to_string = "Advance")]
    Advance,
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
            Self::Sqlite => "sqlite://loco_app.sqlite?mode=rwc",
            Self::Postgres => "postgres://loco:loco@localhost:5432/loco_app",
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
            "{}: You've selected `{}` for your background worker configuration (you should have a \
             Redis/valkey instance to connect to)",
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
            "{}: You've selected `{}` for your asset serving configuration.\n\nNext step, build \
             your frontend:\n  $ cd {}\n  $ npm install && npm run build\n",
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
