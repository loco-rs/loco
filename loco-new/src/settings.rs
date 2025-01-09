//! Defines configurable application settings.

use std::env;

use axum_extra::extract::cookie::Key;
use heck::ToSnakeCase;
use rhai::{CustomType, TypeBuilder};
use serde::{Deserialize, Serialize};

use crate::{
    wizard::{self, AssetsOption, BackgroundOption, DBOption},
    LOCO_VERSION, OS,
};

/// Represents general application settings.
#[derive(Serialize, Deserialize, Clone, Debug, CustomType)]
pub struct Settings {
    pub package_name: String,
    pub module_name: String,
    pub db: Option<Db>,
    pub background: Option<Background>,
    pub asset: Option<Asset>,
    pub auth: bool,
    pub mailer: bool,
    pub clientside: bool,
    pub initializers: Option<Initializers>,
    pub features: Features,
    pub loco_version_text: String,
    pub session_private_key: String,
    pub os: OS,
}

impl From<DBOption> for Option<Db> {
    fn from(db_option: DBOption) -> Self {
        match db_option {
            DBOption::None => None,
            _ => Some(Db {
                kind: db_option.clone(),
                endpoint: db_option.endpoint_config().to_string(),
            }),
        }
    }
}

impl From<BackgroundOption> for Option<Background> {
    fn from(bg: BackgroundOption) -> Self {
        match bg {
            BackgroundOption::None => None,
            _ => Some(Background { kind: bg }),
        }
    }
}

impl From<AssetsOption> for Option<Asset> {
    fn from(asset: AssetsOption) -> Self {
        match asset {
            AssetsOption::None => None,
            _ => Some(Asset { kind: asset }),
        }
    }
}

impl Settings {
    /// Creates a new [`Settings`] instance based on prompt selections.
    #[must_use]
    pub fn from_wizard(package_name: &str, prompt_selection: &wizard::Selections, os: OS) -> Self {
        let features = if prompt_selection.db.enable() {
            Features::default()
        } else {
            let mut features = Features::disable_features();
            if prompt_selection.background.enable() {
                features.names.push("bg_redis".to_string());
            };
            features
        };

        Self {
            package_name: package_name.to_string(),
            module_name: package_name.to_snake_case(),
            auth: prompt_selection.db.enable() && prompt_selection.background.enable(),
            mailer: prompt_selection.db.enable() && prompt_selection.background.enable(),
            db: prompt_selection.db.clone().into(),
            background: prompt_selection.background.clone().into(),
            asset: prompt_selection.asset.clone().into(),
            clientside: prompt_selection.asset.enable(),
            initializers: if prompt_selection.asset.enable() {
                Some(Initializers { view_engine: true })
            } else {
                None
            },
            features,
            loco_version_text: get_loco_version_text(),
            session_private_key: get_session_private_key(),
            os,
        }
    }
}
impl Default for Settings {
    fn default() -> Self {
        #[allow(clippy::default_trait_access)]
        Self {
            package_name: Default::default(),
            module_name: Default::default(),
            db: Default::default(),
            background: Default::default(),
            asset: Default::default(),
            auth: Default::default(),
            mailer: Default::default(),
            clientside: Default::default(),
            initializers: Default::default(),
            features: Default::default(),
            loco_version_text: get_loco_version_text(),
            session_private_key: get_session_private_key(),
            os: Default::default(),
        }
    }
}

fn get_loco_version_text() -> String {
    env::var("LOCO_DEV_MODE_PATH").map_or_else(
        |_| format!(r#"version = "{LOCO_VERSION}""#),
        |path| {
            let path = path.replace('\\', "/");
            format!(r#"version="*", path="{path}""#)
        },
    )
}

fn get_session_private_key() -> String {
    format!("{:?}", Key::generate().master().to_vec())
}

/// Database configuration settings.
#[derive(Serialize, Deserialize, Clone, Debug, Default, CustomType)]
pub struct Db {
    pub kind: DBOption,
    pub endpoint: String,
}

/// Background processing configuration.
#[derive(Serialize, Deserialize, Clone, Debug, Default, CustomType)]
pub struct Background {
    pub kind: BackgroundOption,
}

/// Asset configuration settings.
#[derive(Serialize, Deserialize, Clone, Debug, Default, CustomType)]
pub struct Asset {
    pub kind: AssetsOption,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, CustomType)]
pub struct Initializers {
    pub view_engine: bool,
}

/// Feature configuration, allowing toggling of optional features.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Features {
    pub default_features: bool,
    pub names: Vec<String>,
}

impl Default for Features {
    fn default() -> Self {
        Self {
            default_features: true,
            names: vec![],
        }
    }
}

impl Features {
    /// Disables default features.
    #[must_use]
    pub fn disable_features() -> Self {
        Self {
            default_features: false,
            names: vec!["cli".to_string()],
        }
    }
}
