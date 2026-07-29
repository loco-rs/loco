use std::{collections::BTreeMap, fmt::Write as FmtWrites, fs, path::Path};

use sea_orm_migration::MigratorTrait;

use super::IGNORED_TABLES;
use crate::{
    app::AppContext, cargo_config::CargoConfig, config, doctor, errors::Error, Result as AppResult,
};

struct EntityCmd {
    command: Vec<String>,
    flags: BTreeMap<String, Option<String>>,
}

impl EntityCmd {
    fn new(config: &config::Database) -> Self {
        Self {
            command: vec!["generate".to_string(), "entity".to_string()],
            flags: BTreeMap::from([
                ("--database-url".to_string(), Some(config.uri.clone())),
                (
                    "--ignore-tables".to_string(),
                    Some(IGNORED_TABLES.join(",")),
                ),
                (
                    "--output-dir".to_string(),
                    Some("src/models/_entities".to_string()),
                ),
                ("--with-serde".to_string(), Some("both".to_string())),
                ("--with-copy-enums".to_string(), None),
            ]),
        }
    }

    fn merge_with_config(config: &config::Database, toml_config: &toml::Table) -> Self {
        let mut flags = Self::new(config).flags;

        for (key, value) in toml_config {
            let flag_key = format!("--{}", key.replace('_', "-"));

            // Handle special cases
            match flag_key.as_str() {
                "--output-dir" | "--database-url" => {
                    tracing::warn!(
                        "Ignoring {} configuration from Cargo.toml as it cannot be overridden",
                        key
                    );
                    continue;
                }
                "--ignore-tables" => {
                    if let (Some(current_str), Some(new_value)) = (
                        flags.get_mut(&flag_key).and_then(|c| c.as_mut()),
                        value.as_str(),
                    ) {
                        *current_str = format!("{current_str},{new_value}");
                    }
                    continue;
                }
                _ => {}
            }

            // Handle regular flags
            let flag_value = match value {
                toml::Value::String(s) => Some(s.clone()),
                toml::Value::Boolean(true) => None,
                toml::Value::Boolean(false) => continue,
                _ => Some(value.to_string()),
            };

            flags.insert(flag_key, flag_value);
        }

        Self {
            command: vec!["generate".to_string(), "entity".to_string()],
            flags,
        }
    }

    fn command(&self) -> Vec<&str> {
        let mut args: Vec<&str> = self
            .command
            .iter()
            .map(std::string::String::as_str)
            .collect();
        for (flag, value) in &self.flags {
            args.push(flag.as_str());
            if let Some(val) = value {
                args.push(val.as_str());
            }
        }
        args
    }
}

/// Generate entity model.
/// This function using sea-orm-cli.
///
/// # Errors
///
/// Returns a [`AppResult`] if an error occurs during generate model entity.
pub async fn entities<M: MigratorTrait>(ctx: &AppContext) -> AppResult<String> {
    doctor::check_seaorm_cli()?.to_result()?;
    doctor::check_db(&ctx.config.database).await.to_result()?;

    let flags = CargoConfig::from_current_dir()?
        .get_db_entities()
        .map_or_else(
            || EntityCmd::new(&ctx.config.database),
            |entity_config| {
                tracing::info!(
                    ?entity_config,
                    "Found db.entity configuration in Cargo.toml"
                );
                EntityCmd::merge_with_config(&ctx.config.database, entity_config)
            },
        );

    let out = duct::cmd("sea-orm-cli", &flags.command())
        .stderr_to_stdout()
        .run()
        .map_err(|err| {
            Error::Message(format!(
                "failed to generate entity using sea-orm-cli binary. error details: `{err}`",
            ))
        })?;

    fix_entities()?;

    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

// see https://github.com/SeaQL/sea-orm/pull/1947
// also we are generating an extension module from the get go
fn fix_entities() -> AppResult<()> {
    let dir = fs::read_dir("src/models/_entities")?
        .filter_map(|ent| {
            let ent = ent.unwrap();
            if ent.path().is_file()
                && ent.file_name() != "mod.rs"
                && ent.file_name() != "prelude.rs"
            {
                Some(ent.path())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    // remove activemodel impl from all generated entities, and make note to
    // generate a new extension for those who had it
    let activemodel_exp = "impl ActiveModelBehavior for ActiveModel {}";
    let mut cleaned_entities = Vec::new();
    for file in &dir {
        let content = fs::read_to_string(file)?;
        if content.contains(activemodel_exp) {
            let content = content
                .lines()
                .filter(|line| !line.contains(activemodel_exp))
                .collect::<Vec<_>>()
                .join("\n");
            fs::write(file, content)?;
            cleaned_entities.push(file);
        }
    }

    // generate an empty extension with impl activemodel behavior
    let mut models_mod = fs::read_to_string("src/models/mod.rs")?;
    for entity_file in cleaned_entities {
        let new_file = Path::new("src/models").join(
            entity_file
                .file_name()
                .ok_or_else(|| Error::string("cannot extract file name"))?,
        );

        if !new_file.exists() {
            // Check if the entity has an updated_at field
            let entity_content = fs::read_to_string(entity_file)?;
            let has_updated_at = entity_content.contains("pub updated_at: DateTimeWithTimeZone");

            let module = new_file
                .file_stem()
                .ok_or_else(|| Error::string("cannot extract file stem"))?
                .to_str()
                .ok_or_else(|| Error::string("cannot extract file stem"))?;
            let module_pascal = heck::AsPascalCase(module);

            // Conditionally generate the ActiveModelBehavior implementation
            let before_save_impl = if has_updated_at {
                r"#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(self, _db: &C, insert: bool) -> std::result::Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if !insert && self.updated_at.is_unchanged() {
            let mut this = self;
            this.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now().into());
            Ok(this)
        } else {
            Ok(self)
        }
    }
}"
            } else {
                r"#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(self, _db: &C, _insert: bool) -> std::result::Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        Ok(self)
    }
}"
            };

            fs::write(
                &new_file,
                format!(
                    r"use sea_orm::entity::prelude::*;
pub use super::_entities::{module}::{{ActiveModel, Model, Entity}};
pub type {module_pascal} = Entity;

{before_save_impl}

// implement your read-oriented logic here
impl Model {{}}

// implement your write-oriented logic here
impl ActiveModel {{}}

// implement your custom finders, selectors oriented logic here
impl Entity {{}}
"
                ),
            )?;
            if !models_mod.contains(&format!("mod {module}")) {
                let _ = writeln!(models_mod, "pub mod {module};");
            }
        }
    }

    fs::write("src/models/mod.rs", models_mod)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests_cfg::config::get_database_config;

    #[test]
    fn test_entity_cmd_new() {
        let cmd = EntityCmd::new(&get_database_config());

        let expected = "generate entity --database-url sqlite::memory: --ignore-tables \
            seaql_migrations,pg_loco_queue,sqlt_loco_queue,sqlt_loco_queue_lock --output-dir \
            src/models/_entities --with-copy-enums --with-serde both";
        assert_eq!(cmd.command().join(" "), expected);
    }

    #[test]
    fn test_entity_cmd_merge_with_config() {
        let config_str = r#"
max-connections = "1"
ignore-tables = "table1,table2"
with-serde = "none"
model-extra-derives = "ts_rs::Ts"
"#;
        let config: toml::Table = toml::from_str(config_str).unwrap();

        let cmd = EntityCmd::merge_with_config(&get_database_config(), &config);

        let expected = "generate entity --database-url sqlite::memory: --ignore-tables \
            seaql_migrations,pg_loco_queue,sqlt_loco_queue,sqlt_loco_queue_lock,table1,table2 \
            --max-connections 1 --model-extra-derives ts_rs::Ts --output-dir src/models/_entities \
            --with-copy-enums --with-serde none";
        assert_eq!(cmd.command().join(" "), expected);
    }
}
