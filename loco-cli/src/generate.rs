use std::{
    collections::BTreeMap,
    env, fs,
    io::{Read, Write},
    path::PathBuf,
};

use ignore::WalkBuilder;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use regex::Regex;
use serde::{Deserialize, Serialize};

// Name of generator template that should be existing in each starter folder
const GENERATOR_FILE_NAME: &str = "generator.yaml";

const LIB_NAME_PLACEHOLDER: &str = "{{LibName}}";

#[derive(Debug, Clone, Deserialize, Serialize)]
/// Represents the configuration of a template generator.
pub struct Template {
    /// Description of the template.
    pub description: String,
    /// List of rules for placeholder replacement in the generator.
    pub rules: Option<Vec<TemplateRule>>,
}

#[derive(Debug, Clone)]
/// Represents internal placeholders to be replaced.
pub struct ArgsPlaceholder {
    pub lib_name: String,
}

#[derive(Debug, Clone, Serialize)]
/// Enum representing different kinds of template rules.
pub enum TemplateRuleKind {
    LibName,
    JwtToken,
    Any(String),
}

impl ArgsPlaceholder {
    /// replace strings placeholder with cli arguments.
    /// For example, replace any string that contains {{LibName}} with the given lib name.
    pub fn replace_placeholders(&self, content: &str) -> String {
        content.replace(LIB_NAME_PLACEHOLDER, &self.lib_name)
    }
}

/// Deserialize [`TemplateRuleKind`] for supporting any string replacements
impl<'de> Deserialize<'de> for TemplateRuleKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value: serde_yaml::Value = Deserialize::deserialize(deserializer)?;

        match &value {
            serde_yaml::Value::String(s) => match s.as_str() {
                "LibName" => Ok(Self::LibName),
                "JwtToken" => Ok(Self::JwtToken),
                _ => Ok(Self::Any(s.clone())),
            },
            _ => Err(serde::de::Error::custom("Invalid TemplateRuleKind value")),
        }
    }
}

impl TemplateRuleKind {
    #[must_use]
    /// Get the value from the rule Kind.
    pub fn get_val(&self, args: &ArgsPlaceholder) -> String {
        match self {
            Self::LibName => args.lib_name.to_string(),
            Self::JwtToken => thread_rng()
                .sample_iter(&Alphanumeric)
                .take(20)
                .map(char::from)
                .collect(),
            Self::Any(s) => s.to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
/// Represents a placeholder replacement rule.
pub struct TemplateRule {
    #[serde(with = "serde_regex")]
    /// Pattern to search in the file
    pub pattern: Regex,
    /// The replacement kind
    pub kind: TemplateRuleKind,
    #[serde(with = "serde_regex", skip_serializing)]
    /// List of template generator rule for replacement
    pub file_patterns: Option<Vec<Regex>>,
    pub skip_in_ci: Option<bool>,
}

/// Collects template configurations from files named [`GENERATOR_FILE_NAME`]
/// within the root level directories in the provided path. This function
/// gracefully handles any issues related to the existence or format of the
/// generator files, allowing the code to skip problematic starter templates
/// without returning an error. This approach is designed to avoid negatively
/// impacting users due to faulty template configurations.
///
/// # Errors
/// The code should returns an error only when could get folder collections.
pub fn collect_templates(path: &std::path::PathBuf) -> eyre::Result<BTreeMap<String, Template>> {
    tracing::debug!(
        path = path.display().to_string(),
        "collecting starters template"
    );

    let entries = fs::read_dir(path)?;

    let mut templates = BTreeMap::new();

    // Iterate over the entries and filter out directories
    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            if let Some(starter_folder) = entry.file_name().to_str() {
                let generator_file_path = std::path::Path::new(path)
                    .join(starter_folder)
                    .join(GENERATOR_FILE_NAME);

                let outer_span = tracing::info_span!(
                    "generator",
                    file = generator_file_path.display().to_string()
                );
                let _enter = outer_span.enter();

                tracing::debug!("parsing generator file");

                if !generator_file_path.exists() {
                    tracing::debug!("generator file not found");
                    continue;
                }

                let rdr = match std::fs::File::open(&generator_file_path) {
                    Ok(rdr) => rdr,
                    Err(e) => {
                        tracing::debug!(error = e.to_string(), "could not open generator file");
                        continue;
                    }
                };

                match serde_yaml::from_reader(&rdr) {
                    Ok(t) => templates.insert(starter_folder.to_string(), t),
                    Err(e) => {
                        tracing::debug!(error = e.to_string(), "invalid format");
                        continue;
                    }
                };
            }
        }
    }

    Ok(templates)
}

impl Template {
    /// Generates files based on the given template by recursively applying
    /// template rules to files within the specified path.
    ///
    /// # Description
    /// This method utilizes a parallel file walker to traverse the directory
    /// structure starting from the specified root path (`from`). For each
    /// file encountered, it checks whether the template rules should be
    /// applied based on file patterns. If the rules are applicable and an error
    /// occurs during the application, the error is logged, and the walker
    /// is instructed to quit processing further files in the current
    /// subtree.
    pub fn generate(&self, from: &PathBuf, args: &ArgsPlaceholder) {
        let walker = WalkBuilder::new(from).build();

        let collect_file_patterns = self.get_all_file_patterns();
        for entry in walker.flatten() {
            let path = entry.path();

            if !path.starts_with(from.join("target"))
                && Self::should_run_file(path, Some(&collect_file_patterns))
            {
                if let Err(e) = self.apply_rules(path, args) {
                    tracing::info!(
                        error = e.to_string(),
                        path = path.display().to_string(),
                        "could not run rules placeholder replacement on the file"
                    );
                }
            }
        }

        if let Err(err) = fs::remove_file(from.join(GENERATOR_FILE_NAME)) {
            tracing::debug!(error = err.to_string(), "could not delete generator file");
        }
    }

    fn get_all_file_patterns(&self) -> Vec<Regex> {
        self.rules.as_ref().map_or_else(Vec::new, |rules| {
            rules
                .iter()
                .flat_map(|rule| rule.file_patterns.as_deref().unwrap_or_default())
                .cloned()
                .collect()
        })
    }

    /// Applies the specified rules to the content of a file, updating the file
    /// in-place with the modified content.
    ///
    /// # Description
    /// This method reads the content of the file specified by `file`, applies
    /// each rule from the template to the content, and saves the modified
    /// content back to the same file. The rules are only applied if
    /// the file passes the filtering conditions based on file patterns
    /// associated with each rule. If any rule results in modifications to
    /// the content, the file is updated; otherwise, it remains unchanged.
    fn apply_rules(&self, file: &std::path::Path, args: &ArgsPlaceholder) -> std::io::Result<()> {
        let mut content = String::new();
        fs::File::open(file)?.read_to_string(&mut content)?;

        let mut is_changed = false;
        for rule in &self.rules.clone().unwrap_or_default() {
            if Self::should_run_file(file, rule.file_patterns.as_ref())
                && rule.pattern.is_match(&content)
            {
                if rule.skip_in_ci.unwrap_or(false) && env::var("LOCO_CI_MODE").is_ok() {
                    continue;
                }

                let replace = match rule.kind {
                    TemplateRuleKind::LibName | TemplateRuleKind::JwtToken => {
                        rule.kind.get_val(args)
                    }
                    TemplateRuleKind::Any(_) => args.replace_placeholders(&rule.kind.get_val(args)),
                };
                content = rule.pattern.replace_all(&content, replace).to_string();
                is_changed = true;
            }
        }

        if is_changed {
            let mut modified_file = fs::File::create(file)?;
            modified_file.write_all(content.as_bytes())?;
        }

        Ok(())
    }

    /// Determines whether the template rules should be applied to the given
    /// file path based on a list of regex patterns.
    fn should_run_file(path: &std::path::Path, patterns: Option<&Vec<Regex>>) -> bool {
        if path.is_file() {
            let Some(patterns) = patterns else {
                return true;
            };
            if patterns.is_empty() {
                return true;
            }

            for pattern in patterns {
                if pattern.is_match(&path.display().to_string()) {
                    return true;
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {

    use insta::{assert_debug_snapshot, with_settings};
    use tree_fs;

    use super::*;

    #[test]
    fn can_collect_templates() {
        let yaml_content = r"
        files:
        - path: template-a/generator.yaml
          content: |
            description: template_a
            file_patterns: 
              - rs
              - toml
            rules:
              - pattern: test
                kind: LibName
                file_patterns:
                  - rs
        - path: template-b/generator.yaml
          content: |
            description: template_b
            file_patterns: []
        - path: template-c/generator.yaml
          content: |
            invalid-yaml
        ";

        let tree_res = tree_fs::from_yaml_str(yaml_content).unwrap();

        assert_debug_snapshot!(collect_templates(&tree_res));
    }

    #[allow(clippy::trivial_regex)]
    #[test]
    fn can_generate() {
        let yaml_content = r#"
        files:
        - path: Cargo.toml
          content: | 
            name = "loco_starter"
        - path: test.yaml
          content: | 
            secret = MY_SECRET
        - path: any.yaml
          content: | 
            database:
                uri: {{ get_env(name="DATABASE_URL", default="postgres://loco:loco@localhost:5432/loco_app") }}
        "#;
        let tree_res = tree_fs::from_yaml_str(yaml_content).unwrap();

        let template = Template {
            description: "test template".to_string(),
            rules: Some(vec![
                TemplateRule {
                    pattern: Regex::new("loco.*").unwrap(),
                    kind: TemplateRuleKind::LibName,
                    file_patterns: None,
                    skip_in_ci: None,
                },
                TemplateRule {
                    pattern: Regex::new("MY_SECRET").unwrap(),
                    kind: TemplateRuleKind::JwtToken,
                    file_patterns: None,
                    skip_in_ci: None,
                },
                TemplateRule {
                    pattern: Regex::new("postgres://loco:loco@localhost:5432/loco_app").unwrap(),
                    kind: TemplateRuleKind::Any(
                        "postgres://loco:loco@localhost:5432/{{LibName}}_test".to_string(),
                    ),
                    file_patterns: None,
                    skip_in_ci: None,
                },
            ]),
        };

        let args = ArgsPlaceholder {
            lib_name: "lib_name_changed".to_string(),
        };
        template.generate(&tree_res, &args);

        assert_debug_snapshot!(fs::read_to_string(tree_res.join("Cargo.toml")));

        with_settings!({
            filters => vec![
            (r"([A-Za-z0-9]){20}", "RAND_SECRET"),
            ]
        }, {
            assert_debug_snapshot!(fs::read_to_string(tree_res.join("test.yaml")));
            assert_debug_snapshot!(fs::read_to_string(tree_res.join("any.yaml")));

        });
    }

    #[allow(clippy::trivial_regex)]
    #[test]
    fn can_generate_skip_files() {
        let yaml_content = r#"
        files:
        - path: Cargo.toml
          content: | 
            name = "skip_lib_name_changes"
        - path: test.yaml
          content: | 
            secret = skip_jwt_token
        "#;
        let tree_res = tree_fs::from_yaml_str(yaml_content).unwrap();

        let template = Template {
            description: "test template".to_string(),
            rules: Some(vec![
                TemplateRule {
                    pattern: Regex::new("skip_lib.*").unwrap(),
                    kind: TemplateRuleKind::LibName,
                    file_patterns: None,
                    skip_in_ci: None,
                },
                TemplateRule {
                    pattern: Regex::new("skip_jwt_token").unwrap(),
                    kind: TemplateRuleKind::JwtToken,
                    file_patterns: Some(vec![Regex::new("^*.json").unwrap()]),
                    skip_in_ci: None,
                },
            ]),
        };

        let args = ArgsPlaceholder {
            lib_name: "lib_name_changed".to_string(),
        };
        template.generate(&tree_res, &args);

        assert_debug_snapshot!(fs::read_to_string(tree_res.join("Cargo.toml")));
        assert_debug_snapshot!(fs::read_to_string(tree_res.join("test.yaml")));
    }
}
