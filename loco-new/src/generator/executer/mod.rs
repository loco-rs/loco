//! This module defines error handling and the [`Executer`] trait for file and
//! template operations within the application. It includes custom error types
//! for handling different failure scenarios, including file system and template
//! processing errors.

use crate::settings::Settings;
mod filesystem;
mod inmem;
use std::path::{Path, PathBuf};

pub use filesystem::FileSystem;
pub use inmem::Inmem;
#[cfg(test)]
use mockall::{automock, predicate::*};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Message(String),

    #[error(transparent)]
    TemplateEngine(#[from] Box<rhai::EvalAltResult>),

    #[error(transparent)]
    FS(#[from] fs_extra::error::Error),

    #[error(transparent)]
    Template(#[from] tera::Error),
}
impl Error {
    /// Creates a new error with a custom message.
    pub fn msg<S: Into<String>>(msg: S) -> Self {
        Self::Message(msg.into())
    }
}

#[cfg_attr(test, automock)]
pub trait Executer: Send + Sync {
    /// Copies a single file from the specified path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be copied, such as if the path is
    /// invalid or if a file system error occurs.
    fn copy_file(&self, path: &Path) -> Result<PathBuf>;

    /// Copies a single file from the specified path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be copied, such as if the path is
    /// invalid or if a file system error occurs.
    fn create_file(&self, path: &Path, content: String) -> Result<PathBuf>;

    /// Copies an entire directory from the specified path.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be copied, such as if the path
    /// is invalid or if a file system error occurs.
    fn copy_dir(&self, path: &Path) -> Result<()>;

    /// Copies a template file from the specified path, applying settings.
    ///
    /// # Errors
    ///
    /// Returns an error if the template cannot be copied or if any
    /// settings-related error occurs.
    fn copy_template(&self, path: &Path, data: &Settings) -> Result<()>;

    /// Copies an entire template directory from the specified path, applying
    /// settings.
    ///
    /// # Errors
    ///
    /// Returns an error if the template directory cannot be copied or if any
    /// settings-related error occurs.
    fn copy_template_dir(&self, path: &Path, data: &Settings) -> Result<()>;
}
