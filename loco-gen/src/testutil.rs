//
// generator test toolkit
// to be extracted to a library later.
//
use std::{
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use regex::Regex;

// Define the custom struct to encapsulate file content
pub struct FileContent {
    content: String,
}

impl FileContent {
    // Method to load content from a file into the struct
    pub fn from_file(file_path: &str) -> Result<Self, Box<dyn Error>> {
        let content = fs::read_to_string(file_path)?;
        Ok(Self { content })
    }

    // Method to check that the content contains a specific string
    pub fn check_contains(&self, pattern: &str) -> Result<(), Box<dyn Error>> {
        if self.content.contains(pattern) {
            Ok(())
        } else {
            Err(Box::from(format!("Content does not contain '{pattern}'")))
        }
    }

    // Assert method for check_contains
    pub fn assert_contains(&self, pattern: &str) {
        self.check_contains(pattern)
            .unwrap_or_else(|e| panic!("{}", e));
    }

    // Method to check that the content matches a regular expression
    pub fn check_regex_match(&self, pattern: &str) -> Result<(), Box<dyn Error>> {
        let re = Regex::new(pattern)?;
        if re.is_match(&self.content) {
            Ok(())
        } else {
            Err(Box::from(format!(
                "Content does not match regex '{pattern}'"
            )))
        }
    }

    // Assert method for check_regex_match
    pub fn assert_regex_match(&self, pattern: &str) {
        self.check_regex_match(pattern)
            .unwrap_or_else(|e| panic!("{}", e));
    }

    // Method to check that the content does not contain a specific string
    pub fn check_not_contains(&self, pattern: &str) -> Result<(), Box<dyn Error>> {
        #[allow(clippy::if_not_else)]
        if !self.content.contains(pattern) {
            Ok(())
        } else {
            Err(Box::from(format!("Content should not contain '{pattern}'")))
        }
    }

    // Assert method for check_not_contains
    pub fn assert_not_contains(&self, pattern: &str) {
        self.check_not_contains(pattern)
            .unwrap_or_else(|e| panic!("{}", e));
    }

    // Method to check the length of the content
    pub fn check_length(&self, expected_length: usize) -> Result<(), Box<dyn Error>> {
        if self.content.len() == expected_length {
            Ok(())
        } else {
            Err(Box::from(format!(
                "Content length is {}, expected {}",
                self.content.len(),
                expected_length
            )))
        }
    }

    // Assert method for check_length
    pub fn assert_length(&self, expected_length: usize) {
        self.check_length(expected_length)
            .unwrap_or_else(|e| panic!("{}", e));
    }

    // Method to check the syntax using rustfmt without creating a temp file
    pub fn check_syntax(&self) -> Result<(), Box<dyn Error>> {
        // Parse the file using `syn` to check for valid Rust syntax
        match syn::parse_file(&self.content) {
            Ok(_) => Ok(()),
            Err(err) => Err(Box::from(format!("Syntax error: {err}"))),
        }
    }

    // Assert method for check_syntax
    pub fn assert_syntax(&self) {
        self.check_syntax().unwrap_or_else(|e| panic!("{}", e));
    }
}

// Function that loads the file and applies the provided closure for assertions
pub fn check_file<F>(file_path: &str, assertions: F) -> Result<(), Box<dyn Error>>
where
    F: Fn(&FileContent) -> Result<(), Box<dyn Error>>,
{
    let content = FileContent::from_file(file_path)?;
    assertions(&content)?;
    Ok(())
}

// Assert function for checking the file with a closure for custom assertions
pub fn assert_file<F>(file_path: &str, assertions: F)
where
    F: Fn(&FileContent),
{
    check_file(file_path, |content| {
        assertions(content);
        Ok(())
    })
    .unwrap_or_else(|e| panic!("{}", e));
}

pub fn check_no_warnings() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("cargo")
        .arg("check")
        .arg("--message-format=json")
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    if stdout.contains("warning:") {
        Err(Box::from("Compilation produced warnings"))
    } else {
        Ok(())
    }
}

pub fn assert_no_warnings() {
    check_no_warnings().unwrap_or_else(|e| panic!("{}", e));
}

pub fn check_cargo_check() -> Result<(), Box<dyn Error>> {
    let output = Command::new("cargo").arg("check").output()?; // Execute the command and get the output

    // Check if cargo check was successful
    if output.status.success() {
        Ok(())
    } else {
        // Capture and return the error output if the command failed
        let error_message = String::from_utf8_lossy(&output.stderr);
        Err(Box::from(format!("cargo check failed: {error_message}")))
    }
}

pub fn assert_cargo_check() {
    check_cargo_check().unwrap_or_else(|e| panic!("{}", e));
}

pub fn check_file_not_exists(file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    if std::path::Path::new(file_path).exists() {
        Err(Box::from(format!("File {file_path} should not exist")))
    } else {
        Ok(())
    }
}

pub fn assert_file_not_exists(file_path: &str) {
    check_file_not_exists(file_path).unwrap_or_else(|e| panic!("{}", e));
}

pub fn check_file_exists(file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    if std::path::Path::new(file_path).exists() {
        Ok(())
    } else {
        Err(Box::from(format!("File {file_path} does not exist")))
    }
}

pub fn assert_file_exists(file_path: &str) {
    check_file_exists(file_path).unwrap_or_else(|e| panic!("{}", e));
}

pub fn check_dir_exists(dir_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    if std::path::Path::new(dir_path).is_dir() {
        Ok(())
    } else {
        Err(Box::from(format!("Directory {dir_path} does not exist")))
    }
}

pub fn assert_dir_exists(dir_path: &str) {
    check_dir_exists(dir_path).unwrap_or_else(|e| panic!("{}", e));
}

/// Checks if there exists exactly one file in the given directory whose name
/// matches the provided regex pattern.
pub fn check_single_file_match<P: AsRef<Path>>(
    dir: P,
    pattern: &str,
) -> Result<PathBuf, Box<dyn Error>> {
    // Compile the provided regex pattern
    let re = Regex::new(pattern)?;

    // Filter files that match the regex
    let matched_files: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(std::result::Result::ok)
        .filter_map(|entry| {
            let path = entry.path();

            #[allow(clippy::option_if_let_else)]
            if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
                if re.is_match(file_name) {
                    Some(path) // Return the path if the regex matches
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    // Ensure that there is exactly one match
    match matched_files.len() {
        0 => Err(Box::from("No file found matching the given pattern.")),
        1 => Ok(matched_files.into_iter().next().unwrap()), /* Return the single matching file's */
        // path
        _ => Err(Box::from("More than one file matches the given pattern.")),
    }
}

pub fn assert_single_file_match<P: AsRef<Path>>(dir: P, pattern: &str) -> PathBuf {
    check_single_file_match(dir, pattern).unwrap_or_else(|e| panic!("{}", e))
}

pub fn with_temp_dir<F>(f: F) -> Result<(), Box<dyn Error>>
where
    F: FnOnce(&Path, &Path),
{
    let previous = env::current_dir()?; // Get the current directory
    println!("Current directory: {previous:?}");

    let tree_fs = tree_fs::TreeBuilder::default().drop(true).create()?; // Create a temporary directory
    let current = &tree_fs.root;

    println!("Temporary directory: {current:?}");
    env::set_current_dir(current)?; // Set the current directory to the temp directory

    // Use catch_unwind to handle panics gracefully
    f(previous.as_path(), current); // Execute the provided closure

    // Restore the original directory
    env::set_current_dir(previous)?;

    Ok(())
}
