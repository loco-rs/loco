//! Minimum-version checks for a Loco app's dependencies.
//!
//! Reads the app's `Cargo.lock` (via the `cargo-lock` crate) and compares the
//! resolved versions of "blessed" crates against their minimum requirements.
//! Used by `loco doctor`.

use std::collections::HashMap;
use std::path::Path;

use cargo_lock::Lockfile;
use semver::{Version, VersionReq};
use thiserror::Error;

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
    #[error("Failed to load Cargo.lock: {0}")]
    LockfileError(String),

    #[error("Error with crate {crate_name}: {msg}")]
    CrateError { crate_name: String, msg: String },
}

pub type Result<T> = std::result::Result<T, VersionCheckError>;

/// Load `Cargo.lock` at `lock_path` and check the requested crates against
/// their minimum versions.
///
/// Returns one [`CrateStatus`] per entry in `min_versions`; crates absent from
/// the lockfile are reported as [`VersionStatus::NotFound`].
///
/// # Errors
/// * If the lockfile cannot be read or parsed.
/// * If a minimum-version requirement is not valid semver.
pub fn check_crate_versions(
    lock_path: impl AsRef<Path>,
    min_versions: HashMap<&str, &str>,
) -> Result<Vec<CrateStatus>> {
    let lockfile = Lockfile::load(lock_path.as_ref())
        .map_err(|e| VersionCheckError::LockfileError(e.to_string()))?;

    let mut results = Vec::new();

    for (crate_name, min_version) in min_versions {
        let min_version_req =
            VersionReq::parse(min_version).map_err(|_| VersionCheckError::CrateError {
                crate_name: crate_name.to_string(),
                msg: format!("Invalid minimum version format: {min_version}"),
            })?;

        let status = match lockfile
            .packages
            .iter()
            .find(|pkg| pkg.name.as_str() == crate_name)
        {
            Some(pkg) => {
                // `cargo_lock` re-exports semver; re-parse via our own `semver`
                // for a stable comparison type.
                let version = Version::parse(&pkg.version.to_string()).map_err(|_| {
                    VersionCheckError::CrateError {
                        crate_name: crate_name.to_string(),
                        msg: format!("Invalid version format in Cargo.lock: {}", pkg.version),
                    }
                })?;

                if min_version_req.matches(&version) {
                    VersionStatus::Ok(version.to_string())
                } else {
                    VersionStatus::Invalid {
                        version: version.to_string(),
                        min_version: min_version.to_string(),
                    }
                }
            }
            None => VersionStatus::NotFound,
        };

        results.push(CrateStatus {
            crate_name: crate_name.to_string(),
            status,
        });
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
version = 3

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

        let mut min_versions = HashMap::new();
        min_versions.insert("serde", "1.0.130");
        min_versions.insert("tokio", "1.0");
        min_versions.insert("rand", "0.8.0");

        let mut result = check_crate_versions(tree.root.join("Cargo.lock"), min_versions).unwrap();
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
        // An invalid semver in the lockfile is rejected at load time.
        let cargo_lock_content = r#"
version = 3

[[package]]
name = "serde"
version = "1.0.x"
"#;

        let tree = setup_test_dir(cargo_lock_content);

        let mut min_versions = HashMap::new();
        min_versions.insert("serde", "1.0.0");

        let result = check_crate_versions(tree.root.join("Cargo.lock"), min_versions);
        assert!(matches!(result, Err(VersionCheckError::LockfileError(_))));
    }

    #[test]
    fn test_missing_crate_is_not_found() {
        let cargo_lock_content = r#"
version = 3

[[package]]
name = "serde"
version = "1.0.130"
"#;

        let tree = setup_test_dir(cargo_lock_content);

        let mut min_versions = HashMap::new();
        min_versions.insert("tokio", "1.0.0");

        let result = check_crate_versions(tree.root.join("Cargo.lock"), min_versions).unwrap();
        assert_eq!(
            result,
            vec![CrateStatus {
                crate_name: "tokio".to_string(),
                status: VersionStatus::NotFound,
            }]
        );
    }

    #[test]
    fn test_exact_version_match_for_minimum_requirement() {
        let cargo_lock_content = r#"
version = 3

[[package]]
name = "serde"
version = "1.0.130"
"#;

        let tree = setup_test_dir(cargo_lock_content);

        let mut min_versions = HashMap::new();
        min_versions.insert("serde", "1.0.130");

        let mut result = check_crate_versions(tree.root.join("Cargo.lock"), min_versions).unwrap();
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
version = 3

[[package]]
name = "serde"
version = "1.0.130"
"#;

        let tree = setup_test_dir(cargo_lock_content);

        let min_versions = HashMap::new(); // Empty map

        let result = check_crate_versions(tree.root.join("Cargo.lock"), min_versions).unwrap();
        assert!(result.is_empty());
    }
}
