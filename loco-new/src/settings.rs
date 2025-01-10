//! Defines configurable application settings.

use std::env;

use heck::ToSnakeCase;
use rhai::{CustomType, TypeBuilder};
use serde::{Deserialize, Serialize};

use crate::{
    wizard::{self, BackgroundOption, DBOption, RenderingMethodOption},
    LOCO_VERSION, OS,
};

/// Represents general application settings.
#[derive(Serialize, Deserialize, Clone, Debug, CustomType)]
pub struct Settings {
    pub package_name: String,
    pub module_name: String,
    pub db: Option<Db>,
    pub background: Option<Background>,
    pub rendering_method: Option<RenderingMethod>,
    pub auth: bool,
    pub mailer: bool,
    pub clientside: bool,
    pub initializers: Option<Initializers>,
    pub features: Features,
    pub loco_version_text: String,
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

impl From<RenderingMethodOption> for Option<RenderingMethod> {
    fn from(rendering_method: RenderingMethodOption) -> Self {
        match rendering_method {
            RenderingMethodOption::None => None,
            _ => Some(RenderingMethod {
                kind: rendering_method,
            }),
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

        // we only need the view engine initializer if we are using serverside rendering
        let initializers = if matches!(
            prompt_selection.rendering_method,
            RenderingMethodOption::Serverside
        ) {
            Some(Initializers { view_engine: true })
        } else {
            None
        };
        Self {
            package_name: package_name.to_string(),
            module_name: package_name.to_snake_case(),
            auth: prompt_selection.db.enable() && prompt_selection.background.enable(),
            mailer: prompt_selection.db.enable() && prompt_selection.background.enable(),
            db: prompt_selection.db.clone().into(),
            background: prompt_selection.background.clone().into(),
            rendering_method: prompt_selection.rendering_method.clone().into(),
            clientside: prompt_selection.rendering_method.enable(),
            initializers,
            features,
            loco_version_text: get_loco_version_text(),
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
            rendering_method: Default::default(),
            auth: Default::default(),
            mailer: Default::default(),
            clientside: Default::default(),
            initializers: Default::default(),
            features: Default::default(),
            loco_version_text: get_loco_version_text(),
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

/// Rendering method configuration.
#[derive(Serialize, Deserialize, Clone, Debug, Default, CustomType)]
pub struct RenderingMethod {
    pub kind: RenderingMethodOption,
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
