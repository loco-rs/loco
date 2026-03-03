//! Resolves custom starter template sources for `loco new --from`.
//!
//! Supports local paths, git URLs (with optional `@branch/tag`), and `.zip` archive URLs.
//! Remote sources are cached in `/tmp` using a sanitized key derived from the URL and branch.

use std::{
    io::{self, Read},
    path::{Path, PathBuf},
    process::Command,
};

/// The source of a custom starter template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StarterSource {
    Local(PathBuf),
    Git { url: String, branch: Option<String> },
    Zip(String),
}

impl StarterSource {
    /// Detects the source type from a user-provided string.
    ///
    /// - HTTP/HTTPS URLs ending in `.zip` → `Zip`
    /// - HTTP/HTTPS or SSH git URLs (optionally with `@branch`) → `Git`
    /// - Everything else → `Local`
    #[must_use]
    pub fn detect(from: &str) -> Self {
        if from.starts_with("http://") || from.starts_with("https://") {
            if std::path::Path::new(from)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
            {
                return Self::Zip(from.to_string());
            }
            let (base_url, branch) = split_git_ref(from);
            return Self::Git {
                url: base_url,
                branch,
            };
        }
        if from.starts_with("git@") {
            let (base_url, branch) = split_git_ref(from);
            return Self::Git {
                url: base_url,
                branch,
            };
        }
        Self::Local(PathBuf::from(from))
    }
}

/// Resolves a `--from` value to a local directory path.
///
/// For remote sources, clones or downloads into `/tmp/<cache_key>`.
/// If `refresh` is true, the cached directory is deleted and re-fetched.
///
/// # Errors
///
/// Returns an error if the path doesn't exist, git clone fails, or zip download fails.
pub fn resolve_from(from: &str, refresh: bool) -> io::Result<PathBuf> {
    match StarterSource::detect(from) {
        StarterSource::Local(path) => {
            if !path.exists() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Template directory '{}' not found", path.display()),
                ));
            }
            Ok(path)
        }
        StarterSource::Git { url, branch } => {
            let key = sanitize_cache_key(&url, branch.as_deref());
            let cache_dir = Path::new("/tmp").join(&key);

            if refresh && cache_dir.exists() {
                std::fs::remove_dir_all(&cache_dir)?;
            }
            if !cache_dir.exists() {
                clone_git(&url, branch.as_deref(), &cache_dir)?;
            }

            Ok(cache_dir)
        }
        StarterSource::Zip(url) => {
            let key = sanitize_cache_key(&url, None);
            let cache_dir = Path::new("/tmp").join(&key);

            if refresh && cache_dir.exists() {
                std::fs::remove_dir_all(&cache_dir)?;
            }
            if !cache_dir.exists() {
                std::fs::create_dir_all(&cache_dir)?;
                download_zip(&url, &cache_dir)?;
            }

            Ok(cache_dir)
        }
    }
}

/// Validates that a template directory contains the required `setup.rhai` file.
///
/// # Errors
///
/// Returns an error with a clear message if `setup.rhai` is missing.
pub fn validate_setup_rhai(template_path: &Path) -> io::Result<()> {
    let setup_path = template_path.join("setup.rhai");
    if !setup_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "Template directory '{}' is missing the required 'setup.rhai' file",
                template_path.display()
            ),
        ));
    }
    Ok(())
}

/// Sanitizes a URL and optional branch/tag into a safe `/tmp` cache directory name.
///
/// Example: `("https://github.com/user/repo", Some("v1.2"))` → `"loco_https___github_com_user_repo_v1_2"`
#[must_use]
pub fn sanitize_cache_key(url: &str, branch: Option<&str>) -> String {
    let raw = branch.map_or_else(|| format!("loco_{url}"), |b| format!("loco_{url}_{b}"));
    raw.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Splits a `@branch/tag` suffix from a git URL.
///
/// Only looks for `@` in the path component (not in the `git@host` or `user@` part).
/// Returns `(base_url, Some(branch))` or `(url.to_string(), None)`.
fn split_git_ref(url: &str) -> (String, Option<String>) {
    let path_start = url.find("://").map_or_else(
        || {
            if url.starts_with("git@") {
                // git@github.com:user/repo → skip past colon
                url.find(':').map_or(url.len(), |i| i + 1)
            } else {
                0
            }
        },
        |pos| {
            // https://github.com/user/repo → skip past host
            url[pos + 3..].find('/').map_or(url.len(), |i| pos + 3 + i)
        },
    );

    url[path_start..].rfind('@').map_or_else(
        || (url.to_string(), None),
        |at_offset| {
            let split_pos = path_start + at_offset;
            let base = url[..split_pos].to_string();
            let branch = url[split_pos + 1..].to_string();
            (base, Some(branch))
        },
    )
}

/// Clones a git repository to `target`. Uses `--branch` + `--depth 1` when a ref is given.
fn clone_git(url: &str, branch: Option<&str>, target: &Path) -> io::Result<()> {
    let mut cmd = Command::new("git");
    cmd.arg("clone");
    if let Some(b) = branch {
        cmd.args(["--branch", b, "--depth", "1"]);
    }
    cmd.arg(url).arg(target);

    let status = cmd.status().map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => io::Error::other("git is not installed or not found in PATH"),
        _ => e,
    })?;
    if !status.success() {
        return Err(io::Error::other(format!("git clone failed for '{url}'")));
    }
    Ok(())
}

/// Downloads and extracts a `.zip` archive from `url` into `target`.
fn download_zip(url: &str, target: &Path) -> io::Result<()> {
    let response = ureq::get(url)
        .call()
        .map_err(|e| io::Error::other(format!("Failed to download '{url}': {e}")))?;

    let mut bytes = Vec::new();
    response.into_reader().read_to_end(&mut bytes)?;

    let cursor = io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid zip archive from '{url}': {e}"),
        )
    })?;

    archive
        .extract(target)
        .map_err(|e| io::Error::other(format!("Failed to extract zip from '{url}': {e}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- StarterSource::detect ---

    #[test]
    fn test_detect_local_relative_path() {
        let source = StarterSource::detect("./my-template");
        assert_eq!(source, StarterSource::Local(PathBuf::from("./my-template")));
    }

    #[test]
    fn test_detect_local_absolute_path() {
        let source = StarterSource::detect("/home/user/my-template");
        assert_eq!(
            source,
            StarterSource::Local(PathBuf::from("/home/user/my-template"))
        );
    }

    #[test]
    fn test_detect_https_git_url_no_branch() {
        let source = StarterSource::detect("https://github.com/user/repo");
        assert_eq!(
            source,
            StarterSource::Git {
                url: "https://github.com/user/repo".to_string(),
                branch: None
            }
        );
    }

    #[test]
    fn test_detect_https_git_url_with_branch() {
        let source = StarterSource::detect("https://github.com/user/repo@main");
        assert_eq!(
            source,
            StarterSource::Git {
                url: "https://github.com/user/repo".to_string(),
                branch: Some("main".to_string())
            }
        );
    }

    #[test]
    fn test_detect_https_git_url_with_tag() {
        let source = StarterSource::detect("https://github.com/user/repo@v1.2.3");
        assert_eq!(
            source,
            StarterSource::Git {
                url: "https://github.com/user/repo".to_string(),
                branch: Some("v1.2.3".to_string())
            }
        );
    }

    #[test]
    fn test_detect_zip_url() {
        let source = StarterSource::detect("https://example.com/starter.zip");
        assert_eq!(
            source,
            StarterSource::Zip("https://example.com/starter.zip".to_string())
        );
    }

    #[test]
    fn test_detect_ssh_git_url_no_branch() {
        let source = StarterSource::detect("git@github.com:user/repo");
        assert_eq!(
            source,
            StarterSource::Git {
                url: "git@github.com:user/repo".to_string(),
                branch: None
            }
        );
    }

    #[test]
    fn test_detect_ssh_git_url_with_branch() {
        let source = StarterSource::detect("git@github.com:user/repo@feature-x");
        assert_eq!(
            source,
            StarterSource::Git {
                url: "git@github.com:user/repo".to_string(),
                branch: Some("feature-x".to_string())
            }
        );
    }

    // --- sanitize_cache_key ---

    #[test]
    fn test_sanitize_cache_key_no_branch() {
        let key = sanitize_cache_key("https://github.com/user/repo", None);
        assert_eq!(key, "loco_https___github_com_user_repo");
    }

    #[test]
    fn test_sanitize_cache_key_with_branch() {
        let key = sanitize_cache_key("https://github.com/user/repo", Some("v1.2"));
        assert_eq!(key, "loco_https___github_com_user_repo_v1_2");
    }

    // --- split_git_ref ---

    #[test]
    fn test_split_git_ref_http_with_branch() {
        let (url, branch) = split_git_ref("https://github.com/user/repo@main");
        assert_eq!(url, "https://github.com/user/repo");
        assert_eq!(branch, Some("main".to_string()));
    }

    #[test]
    fn test_split_git_ref_http_no_branch() {
        let (url, branch) = split_git_ref("https://github.com/user/repo");
        assert_eq!(url, "https://github.com/user/repo");
        assert_eq!(branch, None);
    }

    #[test]
    fn test_split_git_ref_ssh_with_branch() {
        let (url, branch) = split_git_ref("git@github.com:user/repo@v2.0");
        assert_eq!(url, "git@github.com:user/repo");
        assert_eq!(branch, Some("v2.0".to_string()));
    }

    #[test]
    fn test_split_git_ref_ssh_no_branch() {
        let (url, branch) = split_git_ref("git@github.com:user/repo");
        assert_eq!(url, "git@github.com:user/repo");
        assert_eq!(branch, None);
    }

    // --- resolve_from (local path) ---

    #[test]
    fn test_resolve_local_valid_path() {
        let tmp = tree_fs::TreeBuilder::default()
            .add_file("setup.rhai", "")
            .create()
            .unwrap();
        let result = resolve_from(tmp.root.to_str().unwrap(), false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), tmp.root);
    }

    #[test]
    fn test_resolve_local_missing_path() {
        let result = resolve_from("/tmp/loco_test_nonexistent_path_xyzabc123", false);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::NotFound);
    }

    // --- validate_setup_rhai ---

    #[test]
    fn test_validate_setup_rhai_present() {
        let tmp = tree_fs::TreeBuilder::default()
            .add_file("setup.rhai", "")
            .create()
            .unwrap();
        assert!(validate_setup_rhai(&tmp.root).is_ok());
    }

    #[test]
    fn test_validate_setup_rhai_missing() {
        let tmp = tree_fs::TreeBuilder::default().create().unwrap();
        let result = validate_setup_rhai(&tmp.root);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert!(err.to_string().contains("setup.rhai"));
    }

    // --- resolve_from (git, no git binary) ---

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_resolve_git_url_fails_when_git_not_installed() {
        let _guard = ENV_LOCK.lock().unwrap();

        let original_path = std::env::var("PATH").unwrap_or_default();
        let empty_dir = tree_fs::TreeBuilder::default().create().unwrap();
        std::env::set_var("PATH", empty_dir.root.to_str().unwrap());

        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let result = resolve_from(&format!("https://github.com/test/repo-{nanos}"), false);

        std::env::set_var("PATH", &original_path);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("git is not installed"),
            "expected 'git is not installed' error"
        );
    }
}
