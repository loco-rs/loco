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
    if !Regex::new("^[a-zA-Z0-9_]+$")?.is_match(&app) {
        eyre::bail!("app name is invalid, illegal characters. keep names simple: myapp or my_app");
    }
    Ok(app)
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
