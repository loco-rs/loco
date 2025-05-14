use std::collections::HashMap;

use semver::{Version, VersionReq};
use thiserror::Error;

use crate::cargo_config::CargoConfig;

#[derive(Debug, PartialEq, Eq, Ord, PartialOrd)]
pub enum VersionStatus {
    NotFound,
    Invalid {
        version: String,
        min_version: String,
    },
    Ok(String),
}

#[derive(Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct CrateStatus {
    pub crate_name: String,
    pub status: VersionStatus,
}

#[derive(Error, Debug)]
pub enum VersionCheckError {
    #[error("Failed to parse Cargo.lock: {0}")]
    ParseError(#[from] toml::de::Error),

    #[error("Error with crate {crate_name}: {msg}")]
    CrateError { crate_name: String, msg: String },
}

pub type Result<T> = std::result::Result<T, VersionCheckError>;

pub fn check_crate_versions(
    lock_file: &CargoConfig,
    min_versions: HashMap<&str, &str>,
) -> Result<Vec<CrateStatus>> {
    let packages = lock_file
        .get_package_array()
        .map_err(|e| VersionCheckError::ParseError(serde::de::Error::custom(e.to_string())))?;

    let mut results = Vec::new();

    for (crate_name, min_version) in min_versions {
        let min_version_req =
            VersionReq::parse(min_version).map_err(|_| VersionCheckError::CrateError {
                crate_name: crate_name.to_string(),
                msg: format!("Invalid minimum version format: {min_version}"),
            })?;

        let mut found = false;
        for package in packages {
            if let Some(name) = package.get("name").and_then(|v| v.as_str()) {
                if name == crate_name {
                    found = true;
                    let version_str =
                        package
                            .get("version")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| VersionCheckError::CrateError {
                                crate_name: crate_name.to_string(),
                                msg: "Invalid version format in Cargo.lock".to_string(),
                            })?;

                    let version =
                        Version::parse(version_str).map_err(|_| VersionCheckError::CrateError {
                            crate_name: crate_name.to_string(),
                            msg: format!("Invalid version format in Cargo.lock: {version_str}"),
                        })?;

                    let status = if min_version_req.matches(&version) {
                        VersionStatus::Ok(version.to_string())
                    } else {
                        VersionStatus::Invalid {
                            version: version.to_string(),
                            min_version: min_version.to_string(),
                        }
                    };
                    results.push(CrateStatus {
                        crate_name: crate_name.to_string(),
                        status,
                    });
                    break;
                }
            }
        }

        if !found {
            results.push(CrateStatus {
                crate_name: crate_name.to_string(),
                status: VersionStatus::NotFound,
            });
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_fs::{Tree, TreeBuilder};

    fn setup_test_dir(cargo_lock_content: &str) -> Tree {
        TreeBuilder::default()
            .add_file("Cargo.lock", cargo_lock_content)
            .create()
            .expect("Failed to create test directory structure")
    }

    #[test]
    fn test_multiple_crates_mixed_results() {
        let cargo_lock_content = r#"
            [[package]]
            name = "serde"
            version = "1.0.130"

            [[package]]
            name = "tokio"
            version = "0.3.0"

            [[package]]
            name = "rand"
            version = "0.8.4"
        "#;

        let tree = setup_test_dir(cargo_lock_content);
        let config = CargoConfig::from_path(tree.root.join("Cargo.lock")).unwrap();

        let mut min_versions = HashMap::new();
        min_versions.insert("serde", "1.0.130");
        min_versions.insert("tokio", "1.0");
        min_versions.insert("rand", "0.8.0");

        let mut result = check_crate_versions(&config, min_versions).unwrap();
        result.sort();
        assert_eq!(
            result,
            vec![
                CrateStatus {
                    crate_name: "rand".to_string(),
                    status: VersionStatus::Ok("0.8.4".to_string())
                },
                CrateStatus {
                    crate_name: "serde".to_string(),
                    status: VersionStatus::Ok("1.0.130".to_string())
                },
                CrateStatus {
                    crate_name: "tokio".to_string(),
                    status: VersionStatus::Invalid {
                        version: "0.3.0".to_string(),
                        min_version: "1.0".to_string()
                    }
                }
            ]
        );
    }

    #[test]
    fn test_invalid_version_format_in_cargo_lock() {
        let cargo_lock_content = r#"
            [[package]]
            name = "serde"
            version = "1.0.x"
        "#;

        let tree = setup_test_dir(cargo_lock_content);
        let config = CargoConfig::from_path(tree.root.join("Cargo.lock")).unwrap();

        let mut min_versions = HashMap::new();
        min_versions.insert("serde", "1.0.0");

        let result = check_crate_versions(&config, min_versions);
        assert!(matches!(
            result,
            Err(VersionCheckError::CrateError { crate_name, msg }) if crate_name == "serde" && msg.contains("Invalid version format")
        ));
    }

    #[test]
    fn test_no_package_section_in_cargo_lock() {
        let cargo_lock_content = r"
            # No packages listed in this Cargo.lock
        ";

        let tree = setup_test_dir(cargo_lock_content);
        let config = CargoConfig::from_path(tree.root.join("Cargo.lock")).unwrap();

        let mut min_versions = HashMap::new();
        min_versions.insert("serde", "1.0.130");

        let result = check_crate_versions(&config, min_versions);
        assert!(matches!(result, Err(VersionCheckError::ParseError(_))));
    }

    #[test]
    fn test_exact_version_match_for_minimum_requirement() {
        let cargo_lock_content = r#"
            [[package]]
            name = "serde"
            version = "1.0.130"
        "#;

        let tree = setup_test_dir(cargo_lock_content);
        let config = CargoConfig::from_path(tree.root.join("Cargo.lock")).unwrap();

        let mut min_versions = HashMap::new();
        min_versions.insert("serde", "1.0.130");

        let mut result = check_crate_versions(&config, min_versions).unwrap();
        result.sort();
        assert_eq!(
            result,
            vec![CrateStatus {
                crate_name: "serde".to_string(),
                status: VersionStatus::Ok("1.0.130".to_string()),
            }]
        );
    }

    #[test]
    fn test_no_crates_in_min_versions_map() {
        let cargo_lock_content = r#"
            [[package]]
            name = "serde"
            version = "1.0.130"
        "#;

        let tree = setup_test_dir(cargo_lock_content);
        let config = CargoConfig::from_path(tree.root.join("Cargo.lock")).unwrap();

        let min_versions = HashMap::new(); // Empty map

        let result = check_crate_versions(&config, min_versions).unwrap();
        assert!(result.is_empty());
    }
}
