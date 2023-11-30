use std::collections::HashMap;

use dialoguer::{theme::ColorfulTheme, Input, Select};
use regex::Regex;

const SAAS_STARTER: &str = "https://github.com/loco-rs/saas-starter-template";
const STATELESS_STARTER: &str = "https://github.com/loco-rs/stateless-starter-template";

const OPTIONS: &[(&str, &str, &str)] = &[
    ("saas", "Saas app (with DB and user auth)", SAAS_STARTER),
    (
        "stateless",
        "Stateless service (minimal, no db)",
        STATELESS_STARTER,
    ),
];

pub fn prompt_app() -> eyre::Result<String> {
    let app = Input::new()
        .with_prompt("❯ App name?")
        .default("myapp".to_string())
        .interact_text()?;
    validate_app_name(&app)?;
    Ok(app)
}

pub fn validate_app_name(app: &str) -> eyre::Result<()> {
    if !Regex::new("^[a-zA-Z0-9_]+$")?.is_match(app) {
        eyre::bail!("app name is invalid, illegal characters. keep names simple: myapp or my_app");
    }

    Ok(())
}

pub fn prompt_selection() -> eyre::Result<String> {
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("❯ What would you like to build?")
        .default(0)
        .items(&OPTIONS.iter().map(|opt| opt.1).collect::<Vec<_>>())
        .interact()?;

    println!("{}", &OPTIONS[selection].1);
    Ok(OPTIONS[selection].2.to_string())
}

pub fn get_template_url_by_name(name: &str) -> eyre::Result<String> {
    let options_map: HashMap<_, _> = OPTIONS
        .iter()
        .map(|&(key, _, value)| (key, value))
        .collect();

    let val = options_map
        .get(name)
        .ok_or(eyre::eyre!("template not found"))?;

    Ok((*val).to_string())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn can_get_template_url_by_name() {
        assert_eq!(
            get_template_url_by_name("saas").ok(),
            Some(SAAS_STARTER.to_string())
        );
        assert_eq!(
            get_template_url_by_name("stateless").ok(),
            Some(STATELESS_STARTER.to_string())
        );
        assert!(get_template_url_by_name("invalid").is_err(),);
    }

    #[test]
    fn can_validate_app_name() {
        assert!(validate_app_name("myapp").is_ok());
        assert!(validate_app_name("myapp_webserver").is_ok());
        assert!(validate_app_name("myapp-webserver").is_err());
    }
}
