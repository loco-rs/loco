use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use strum::Display;

pub mod generator;
pub mod settings;
pub mod wizard;

pub type Result<T> = std::result::Result<T, Error>;

/// Matching minimal Loco version.
pub const LOCO_VERSION: &str = "0.16";

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Message(String),

    #[error(transparent)]
    Dialog(#[from] dialoguer::Error),

    #[error(transparent)]
    IO(#[from] std::io::Error),

    #[error(transparent)]
    FS(#[from] fs_extra::error::Error),

    #[error(transparent)]
    TemplateEngine(#[from] Box<rhai::EvalAltResult>),

    #[error(transparent)]
    Generator(#[from] crate::generator::executer::Error),
}
impl Error {
    pub fn msg<S: Into<String>>(msg: S) -> Self {
        Self::Message(msg.into())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Display, Default, PartialEq, Eq, ValueEnum)]
pub enum OS {
    #[cfg_attr(windows, default)]
    #[serde(rename = "windows")]
    Windows,

    #[cfg_attr(unix, default)]
    #[serde(rename = "linux")]
    Linux,

    #[serde(rename = "macos")]
    Macos,
}
