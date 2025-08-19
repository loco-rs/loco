use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::OnceLock,
};

use cargo_metadata::semver::Version;
use colored::Colorize;
use regex::Regex;

use crate::{
    ci,
    errors::{Error, Result},
    out, utils,
};

static REPLACE_LOCO_LIB_VERSION_: OnceLock<Regex> = OnceLock::new();
static REPLACE_LOCO_PACKAGE_VERSION: OnceLock<Regex> = OnceLock::new();

fn get_replace_loco_lib_version() -> &'static Regex {
    REPLACE_LOCO_LIB_VERSION_.get_or_init(|| {
        Regex::new(
            r#"(?P<name>name\s*=\s*".+\s+version\s*=\s*")(?P<version>[0-9]+\.[0-9]+\.[0-9]+)"#,
        )
        .unwrap()
    })
}

fn get_replace_loco_package_version() -> &'static Regex {
    REPLACE_LOCO_PACKAGE_VERSION
        .get_or_init(|| Regex::new(r#"loco-rs = \{ (version|path) = "[^"]+""#).unwrap())
}
pub struct BumpVersion {
    pub base_dir: PathBuf,
    pub version: Version,
    pub bump_starters: bool,
}

impl BumpVersion {
    /// Bump all necessary loco resources with the given version.
    ///
    /// # Errors
    /// Returns an error when it could not update one of the resources.
    pub fn run(&self) -> Result<()> {
        self.bump_loco_framework(".")?;
        self.bump_loco_framework("loco-gen")?;
        self.bump_subcrates_version(&["loco-gen"])?;

        // change starters from fixed (v0.1.x) to local ("../../") in order
        // to test all starters against what is going to be released
        // when finished successfully, you're allowed to bump all starters to the new
        // version
        if self.bump_starters {
            self.modify_starters_loco_version("loco-rs = { path = \"../../\"")?;

            println!("Testing starters CI");

            let starter_projects: Vec<ci::RunResults> =
                ci::run_all_in_folder(&self.base_dir.join(utils::FOLDER_STARTERS))?;

            println!("Starters CI results:");
            println!("{}", out::print_ci_results(&starter_projects));
            for starter in &starter_projects {
                if !starter.is_valid() {
                    return Err(Error::Message(format!(
                        "starter {} ins not passing the CI",
                        starter.path.display()
                    )));
                }
            }

            self.modify_starters_loco_version(&format!(
                "loco-rs = {{ version = \"{}\"",
                self.version
            ))?;
            println!("{}", "Bump loco starters finished successfully".green());
        }

        Ok(())
    }

    /// Bump the version of the loco library in the root package's Cargo.toml
    /// file.
    ///
    /// # Errors
    /// Returns an error when it could not parse the loco Cargo.toml file or has
    /// an error updating the file.
    fn bump_loco_framework(&self, path: &str) -> Result<()> {
        println!("bumping to `{}` on `{path}`", self.version);

        let mut content = String::new();

        let cargo_toml_file = self.base_dir.join(path).join("Cargo.toml");
        fs::File::open(&cargo_toml_file)?.read_to_string(&mut content)?;

        if !get_replace_loco_lib_version().is_match(&content) {
            return Err(Error::BumpVersion {
                path: cargo_toml_file,
                package: "root_package".to_string(),
            });
        }

        let content = get_replace_loco_lib_version()
            .replace(&content, |captures: &regex::Captures<'_>| {
                format!("{}{}", &captures["name"], self.version)
            });

        let mut modified_file = fs::File::create(cargo_toml_file)?;
        modified_file.write_all(content.as_bytes())?;

        Ok(())
    }

    fn bump_subcrates_version(&self, crates: &[&str]) -> Result<()> {
        let mut content = String::new();

        let cargo_toml_file = self.base_dir.join("Cargo.toml");
        fs::File::open(&cargo_toml_file)?.read_to_string(&mut content)?;

        println!("in root package:");
        for subcrate in crates {
            println!("bumping subcrate `{}` to `{}`", subcrate, self.version);
            let re = Regex::new(&format!(
                r#"{subcrate}\s*=\s*\{{\s*version\s*=\s*"[0-9]+\.[0-9]+\.[0-9]+",\s*path\s*=\s*"[^"]+"\s*\}}"#,
            ))
            .unwrap();

            if !re.is_match(&content) {
                return Err(Error::BumpVersion {
                    path: cargo_toml_file.clone(),
                    package: subcrate.to_string(),
                });
            }

            // Replace the full version line with the new version, keeping the structure
            // intact
            content = re
                .replace(
                    &content,
                    format!(
                        r#"{subcrate} = {{ version = "{}", path = "./{subcrate}" }}"#,
                        self.version
                    ),
                )
                .to_string();
        }

        let mut modified_file = fs::File::create(cargo_toml_file)?;
        modified_file.write_all(content.as_bytes())?;
        Ok(())
    }

    /// Update the dependencies of loco-rs in all starter projects to the given
    /// version.
    ///
    /// # Errors
    /// Returns an error when it could not parse a loco Cargo.toml file or has
    /// an error updating the file.
    pub fn modify_starters_loco_version(&self, replace_with: &str) -> Result<()> {
        let starter_projects =
            utils::get_cargo_folders(&self.base_dir.join(utils::FOLDER_STARTERS))?;

        for starter_project in starter_projects {
            Self::replace_loco_rs_version(&starter_project, replace_with)?;
        }

        Ok(())
    }

    fn replace_loco_rs_version(path: &Path, replace_with: &str) -> Result<()> {
        let mut content = String::new();
        let cargo_toml_file = path.join("Cargo.toml");
        fs::File::open(&cargo_toml_file)?.read_to_string(&mut content)?;

        if !get_replace_loco_package_version().is_match(&content) {
            return Err(Error::BumpVersion {
                path: cargo_toml_file,
                package: "loco-rs".to_string(),
            });
        }
        content = get_replace_loco_package_version()
            .replace_all(&content, |_captures: &regex::Captures<'_>| {
                replace_with.to_string()
            })
            .to_string();

        let mut modified_file = fs::File::create(cargo_toml_file)?;
        modified_file.write_all(content.as_bytes())?;
        Ok(())
    }
}
