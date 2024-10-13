use super::generate_project::GenerateProjectStep;
use crazy_train::{executer, step, Randomizer, Result, StringDef};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const SCAFFOLD_MAPPING: &str = include_str!("../../../src/gen/mappings.json");

#[derive(Serialize, Deserialize, Debug)]
struct FieldType {
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Mappings {
    field_types: Vec<FieldType>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ScaffoldStep {
    pub template_location: PathBuf,
    pub fields: Vec<String>,
    pub rand_table_name: bool,
    pub rand_fields_name: bool,
}

impl ScaffoldStep {
    fn new(template_location: &Path, rand_table_name: bool, rand_fields_name: bool) -> Self {
        let field_mapping: Mappings = serde_yaml::from_str(SCAFFOLD_MAPPING).expect("mapping");

        let fields = field_mapping
            .field_types
            .iter()
            .map(|t| t.name.clone())
            .collect::<Vec<_>>();

        Self {
            template_location: template_location.to_path_buf(),
            fields,
            rand_fields_name,
            rand_table_name,
        }
    }
}

impl step::StepTrait for ScaffoldStep {
    fn plan(&self, randomizer: &Randomizer) -> Result<step::Plan> {
        let table_name = if self.rand_table_name {
            randomizer.string(StringDef::from_randomizer(randomizer))
        } else {
            randomizer.string(StringDef::default())
        }
        .to_string();

        let shuffled_fields = randomizer.shuffle(&self.fields);
        let random_fields = randomizer.pick_random(&shuffled_fields);

        let fields = random_fields
            .iter()
            .map(|kind| {
                format!(
                    "'{}:{kind}'",
                    if self.rand_fields_name {
                        randomizer.string(StringDef::from_randomizer(randomizer))
                    } else {
                        randomizer.string(StringDef::default())
                    }
                )
            })
            .collect::<Vec<_>>();

        let command = format!(
            "cd {} && cargo loco generate scaffold {} {} --api",
            self.template_location.display(),
            table_name,
            fields.join(" ")
        );
        Ok(step::Plan {
            id: std::any::type_name::<Self>().to_string(),
            command,
        })
    }

    fn is_success(
        &self,
        execution_result: &executer::Output,
    ) -> std::result::Result<bool, &'static str> {
        if execution_result.status_code == Some(1) {
            Ok(false)
        } else {
            Ok(true)
        }
    }

    fn run_check(&self) -> Option<String> {
        Some(format!(
            "cd {} && cargo check",
            self.template_location.display()
        ))
    }

    fn run_test(&self) -> Option<String> {
        Some(format!(
            "cd {} && cargo test",
            self.template_location.display()
        ))
    }

    fn to_yaml(&self) -> serde_yaml::Value {
        serde_yaml::to_value(self).expect("to yaml")
    }
}

pub fn run(randomizer: Randomizer, temp_dir: &Path) -> crazy_train::Runner {
    let template_step =
        GenerateProjectStep::new(&randomizer, temp_dir, Some("test_scaffold"), false, false);
    let scaffold_step = ScaffoldStep::new(
        template_step
            .location
            .join(&template_step.project_name)
            .as_path(),
        true,
        true,
    );

    crazy_train::new(vec![Box::new(template_step), Box::new(scaffold_step)]).randomizer(randomizer)
}
