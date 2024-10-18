use crazy_train::{executer, step, Randomizer, StringDef};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug)]
pub struct GenerateProjectStep {
    pub location: PathBuf,
    pub project_name: String,
    pub run_check: bool,
    pub run_test: bool,
}

impl GenerateProjectStep {
    pub fn new(
        randomizer: &Randomizer,
        root_dir: &Path,
        project_name: Option<&str>,
        run_check: bool,
        run_test: bool,
    ) -> Self {
        let project_name = project_name.map_or_else(
            || {
                randomizer
                    .string(StringDef::from_randomizer(randomizer))
                    .to_string()
            },
            std::string::ToString::to_string,
        );

        Self {
            location: root_dir.join(randomizer.path()),
            project_name,
            run_check,
            run_test,
        }
    }
}

impl step::StepTrait for GenerateProjectStep {
    fn setup(&self) -> crazy_train::Result<()> {
        Ok(std::fs::create_dir_all(&self.location)?)
    }

    fn plan(&self, _randomizer: &Randomizer) -> crazy_train::Result<step::Plan> {
        // TODO:: --template and --assets should be random also
        let escaped_project_name =
            shell_escape::escape(self.project_name.clone().into()).to_string();
        let command = format!("loco new --name {} --template saas --db sqlite --bg async --assets serverside --path {}", escaped_project_name,self.location.display());

        Ok(step::Plan {
            id: std::any::type_name::<Self>().to_string(),
            command,
        })
    }

    fn is_success(
        &self,
        execution_result: &executer::Output,
    ) -> std::result::Result<bool, &'static str> {
        let re_invalid_project_name = Regex::new(
            r"(the first character must be a|characters must be Unicode XID characters|the name cannot start with a digit)",
        )
        .unwrap();
        let re_folder_exists = Regex::new(r"🙀 The specified path '.*.' already exist\n").unwrap();
        let re_successfully = Regex::new(r"\n🚂 Loco app generated successfully in:\n.*").unwrap();

        if StringDef::contains_unicode(&self.project_name)
            || self
                .project_name
                .chars()
                .any(|c| StringDef::contains_symbols(&format!("{c}")) && c != '_')
        {
            if execution_result.status_code != Some(1) {
                return Err("expected status code 1");
            } else if !re_invalid_project_name.is_match(&execution_result.stderr) {
                return Err("stderr not match to the error pattern");
            } else if !execution_result.stdout.is_empty() {
                return Err("stdout should be empty");
            }
            Ok(false)
        } else if re_folder_exists.is_match(&execution_result.stderr) {
            if execution_result.status_code != Some(1) {
                return Err("when folder exists expected to get exit code 1");
            }
            Ok(false)
        } else if execution_result.status_code == Some(0) {
            if re_successfully.is_match(&execution_result.stderr) {
                Ok(true)
            } else {
                return Err("command success with unexpected stderr");
            }
        } else {
            Err("error not handled")
        }
    }

    fn run_check(&self) -> Option<String> {
        if self.run_check {
            Some(format!(
                "cd {} && cargo check",
                self.location.join(&self.project_name).display()
            ))
        } else {
            None
        }
    }

    fn run_test(&self) -> Option<String> {
        if self.run_check {
            Some(format!(
                "cd {} && cargo test",
                self.location.join(&self.project_name).display()
            ))
        } else {
            None
        }
    }
    fn to_yaml(&self) -> serde_yaml::Value {
        serde_yaml::to_value(self).expect("to yaml")
    }
}

pub fn run(randomizer: Randomizer, temp_dir: &Path) -> crazy_train::Runner {
    let step = GenerateProjectStep::new(&randomizer, temp_dir, None, true, true);
    crazy_train::new(vec![Box::new(step)]).randomizer(randomizer)
}
