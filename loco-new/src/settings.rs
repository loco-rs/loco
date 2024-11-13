//! Defines configurable application settings.

use std::env;

use heck::ToSnakeCase;
use rhai::{CustomType, TypeBuilder};
use serde::{Deserialize, Serialize};

use crate::{wizard, wizard_opts, LOCO_VERSION};

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
    pub initializers: Option<Initializers>,
    pub features: Features,
    pub loco_version_text: String,
}

impl From<wizard_opts::DBOption> for Option<Db> {
    fn from(db_option: wizard_opts::DBOption) -> Self {
        match db_option {
            wizard_opts::DBOption::None => None,
            _ => Some(Db {
                kind: db_option.clone(),
                endpoint: db_option.endpoint_config().to_string(),
            }),
        }
    }
}

impl From<wizard_opts::BackgroundOption> for Option<Background> {
    fn from(bg: wizard_opts::BackgroundOption) -> Self {
        match bg {
            wizard_opts::BackgroundOption::None => None,
            _ => Some(Background { kind: bg }),
        }
    }
}

impl From<wizard_opts::AssetsOption> for Option<Asset> {
    fn from(asset: wizard_opts::AssetsOption) -> Self {
        match asset {
            wizard_opts::AssetsOption::None => None,
            _ => Some(Asset { kind: asset }),
        }
    }
}

impl Settings {
    /// Creates a new [`Settings`] instance based on prompt selections.
    #[must_use]
    pub fn from_wizard(package_name: &str, prompt_selection: &wizard::Selections) -> Self {
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
            auth: prompt_selection.db.enable(),
            mailer: prompt_selection.db.enable(),
            db: prompt_selection.db.clone().into(),
            background: prompt_selection.background.clone().into(),
            asset: prompt_selection.asset.clone().into(),
            initializers: if prompt_selection.asset.enable() {
                Some(Initializers { view_engine: true })
            } else {
                None
            },
            features,
            loco_version_text: get_loco_version_text(),
        }
    }
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            package_name: Default::default(),
            module_name: Default::default(),
            db: Default::default(),
            background: Default::default(),
            asset: Default::default(),
            auth: Default::default(),
            mailer: Default::default(),
            initializers: Default::default(),
            features: Default::default(),
            loco_version_text: get_loco_version_text(),
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

/// Database configuration settings.
#[derive(Serialize, Deserialize, Clone, Debug, Default, CustomType)]
pub struct Db {
    pub kind: wizard_opts::DBOption,
    pub endpoint: String,
}

/// Background processing configuration.
#[derive(Serialize, Deserialize, Clone, Debug, Default, CustomType)]
pub struct Background {
    pub kind: wizard_opts::BackgroundOption,
}

/// Asset configuration settings.
#[derive(Serialize, Deserialize, Clone, Debug, Default, CustomType)]
pub struct Asset {
    pub kind: wizard_opts::AssetsOption,
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
