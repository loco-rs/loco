use std::path::{Path, PathBuf};

use include_dir::{include_dir, Dir, DirEntry, File};

use crate::{Error, Result};

static TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/templates");
pub const DEFAULT_LOCAL_TEMPLATE: &str = ".loco-templates";

/// Returns a list of paths that should be ignored during file collection.
#[must_use]
pub fn get_ignored_paths() -> Vec<&'static Path> {
    vec![
        #[cfg(not(feature = "with-db"))]
        Path::new("scaffold"),
        #[cfg(not(feature = "with-db"))]
        Path::new("migration"),
        #[cfg(not(feature = "with-db"))]
        Path::new("model"),
    ]
}

/// Checks whether a specific path exists in the included templates.
#[must_use]
pub fn exists(path: &Path) -> bool {
    TEMPLATES.get_entry(path).is_some()
}

/// Determines whether a given path should be ignored based on the ignored paths
/// list.
#[must_use]
fn is_path_ignored(path: &Path, ignored_paths: &[&Path]) -> bool {
    ignored_paths
        .iter()
        .any(|&ignored| path.starts_with(ignored))
}

/// Collects all file paths from the included templates directory recursively.
#[must_use]
pub fn collect() -> Vec<PathBuf> {
    collect_files_path_recursively(&TEMPLATES)
}

/// Collects all files from the included templates directory recursively.
#[must_use]
pub fn collect_files() -> Vec<&'static File<'static>> {
    collect_files_recursively(&TEMPLATES)
}

/// Collects all file paths within a specific directory in the templates.
///
/// # Errors
/// Returns [`Error::TemplateNotFound`] if the directory is not found.
pub fn collect_files_path(path: &Path) -> Result<Vec<PathBuf>> {
    TEMPLATES.get_entry(path).map_or_else(
        || {
            Err(Error::TemplateNotFound {
                path: path.to_path_buf(),
            })
        },
        |entry| match entry {
            DirEntry::Dir(dir) => Ok(collect_files_path_recursively(dir)),
            DirEntry::File(file) => Ok(vec![file.path().to_path_buf()]),
        },
    )
}

/// Collects all files within a specific directory in the templates.
///
/// # Errors
/// Returns [`Error::TemplateNotFound`] if the directory is not found.
pub fn collect_files_from_path(path: &Path) -> Result<Vec<&File<'_>>> {
    TEMPLATES.get_entry(path).map_or_else(
        || {
            Err(Error::TemplateNotFound {
                path: path.to_path_buf(),
            })
        },
        |entry| match entry {
            DirEntry::Dir(dir) => Ok(collect_files_recursively(dir)),
            DirEntry::File(file) => Ok(vec![file]),
        },
    )
}

/// Recursively collects all file paths from a directory, skipping ignored
/// paths.
fn collect_files_path_recursively(dir: &Dir<'_>) -> Vec<PathBuf> {
    let mut file_paths = Vec::new();

    for entry in dir.entries() {
        match entry {
            DirEntry::File(file) => file_paths.push(file.path().to_path_buf()),
            DirEntry::Dir(subdir) => {
                if !is_path_ignored(subdir.path(), &get_ignored_paths()) {
                    file_paths.extend(collect_files_path_recursively(subdir));
                }
            }
        }
    }
    file_paths
}

/// Recursively collects all files from a directory, skipping ignored paths.
fn collect_files_recursively<'a>(dir: &'a Dir<'a>) -> Vec<&'a File<'a>> {
    let mut files = Vec::new();

    for entry in dir.entries() {
        match entry {
            DirEntry::File(file) => files.push(file),
            DirEntry::Dir(subdir) => {
                if !is_path_ignored(subdir.path(), &get_ignored_paths()) {
                    files.extend(collect_files_recursively(subdir));
                }
            }
        }
    }
    files
}

#[cfg(test)]
pub mod tests {
    use std::path::Path;

    use super::*;

    /// Returns the first directory in the included templates.
    /// # Panics
    #[must_use]
    pub fn find_first_dir() -> &'static Dir<'static> {
        TEMPLATES.dirs().next().expect("first folder")
    }
    #[must_use]
    pub fn find_first_file<'a>(dir: &'a Dir<'a>) -> Option<&'a File<'a>> {
        for entry in dir.entries() {
            match entry {
                DirEntry::File(file) => return Some(file),
                DirEntry::Dir(sub_dir) => {
                    if let Some(file) = find_first_file(sub_dir) {
                        return Some(file);
                    }
                }
            }
        }
        None
    }

    #[test]
    fn test_get_ignored_paths() {
        let ignored_paths = get_ignored_paths();
        #[cfg(not(feature = "with-db"))]
        {
            assert!(ignored_paths.contains(&Path::new("scaffold")));
            assert!(ignored_paths.contains(&Path::new("migration")));
            assert!(ignored_paths.contains(&Path::new("model")));
        }
        #[cfg(feature = "with-db")]
        {
            assert!(ignored_paths.is_empty());
        }
    }

    #[test]
    fn test_exists() {
        // test existing folder
        let test_folder = TEMPLATES.dirs().next().expect("first folder");
        assert!(exists(test_folder.path()));
        assert!(!exists(Path::new("none-folder")));

        // test existing file
        let test_file = find_first_file(&TEMPLATES).expect("find file");
        println!("==== {:#?}", test_file.path());
        assert!(exists(test_file.path()));
        assert!(!exists(Path::new("none.rs.t")));
    }

    #[test]
    fn test_collect() {
        let file_paths = collect();
        assert!(!file_paths.is_empty());
        for path in file_paths {
            assert!(TEMPLATES.get_entry(&path).is_some());
        }
    }

    #[test]
    fn test_collect_files() {
        let files = collect_files();
        assert!(!files.is_empty());
        for file in files {
            assert!(TEMPLATES.get_entry(file.path()).is_some());
        }
    }

    #[test]
    fn test_is_path_ignored() {
        let path = Path::new("/home/user/project/src/main.rs");
        let ignores = vec![
            Path::new("/home/user/project/target"),
            Path::new("/home/user/project/src"),
        ];

        assert!(is_path_ignored(path, &ignores));

        let non_ignored_path = Path::new("/home/user/project/docs/readme.md");
        assert!(!is_path_ignored(non_ignored_path, &ignores));

        let empty_ignores: &[&Path] = &[];
        assert!(!is_path_ignored(path, empty_ignores));
    }
}
