use std::str::FromStr;

use dialoguer::{theme::ColorfulTheme, Select};
use serde_derive::{Deserialize, Serialize};
use serde_variant::to_variant_name;
use strum::{EnumIter, EnumString, IntoEnumIterator};

#[derive(clap::ValueEnum, Clone, Serialize, Deserialize, Debug, EnumIter, EnumString)]
pub enum Starter {
    #[strum(serialize = "Saas")]
    Saas,
    #[strum(serialize = "Stateless (not db)")]
    Stateless,
}

impl std::fmt::Display for Starter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        to_variant_name(self).expect("only enum supported").fmt(f)
    }
}

impl Starter {
    #[must_use]
    pub fn to_list() -> Vec<String> {
        Self::iter().map(|provider| provider.to_string()).collect()
    }

    /// Show interactive starter selection
    ///
    /// # Errors
    /// When could not show the prompt or could not convert selection the
    /// [`Starter`]
    pub fn prompt_selection() -> eyre::Result<Self> {
        let selections = Self::to_list();
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Choose starter template")
            .default(0)
            .items(&selections[..])
            .interact()?;

        println!("{}", &selections[selection]);
        Ok(Self::from_str(&selections[selection])?)
    }

    #[must_use]
    pub fn git_url(&self) -> String {
        match self {
            Self::Saas => {
                "https://github.com/rustyrails-rs/rustyrails-starter-template".to_string()
            }
            Self::Stateless => {
                "https://github.com/rustyrails-rs/rustyrails-starter-stateless-template".to_string()
            }
        }
    }
}
