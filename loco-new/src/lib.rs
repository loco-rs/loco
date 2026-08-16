use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use strum::Display;

pub mod generator;
pub mod settings;
pub mod wizard;

pub type Result<T> = std::result::Result<T, Error>;

/// Minimum `loco-rs` version a generated app requires.
///
/// This is a **compatibility floor, not a display string**: it becomes the
/// `loco-rs` version requirement in every generated `Cargo.toml`, and cargo
/// resolves it against what is *published*, not against this working tree. So
/// it must always name the release this CLI generates for — `major.minor` of
/// the `loco-rs` version in the workspace manifest, enforced by the test below.
///
/// Leaving it behind does not fail loudly, it ships a broken app: a floor of
/// `"1.0"` while the CLI emits 1.1 templates lets cargo pick the newest
/// published 1.0.x, which renders the YAML-safe `<%= ... %>` config delimiters
/// literally and then fails to parse them. The app compiles and cannot boot.
///
/// No generator test can catch this, which is why the check is here: they all
/// set `LOCO_DEV_MODE_PATH`, which replaces this requirement with a path
/// dependency on the working tree and makes the floor invisible.
pub const LOCO_VERSION: &str = "1.1";

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

#[cfg(test)]
mod tests {
    use super::LOCO_VERSION;

    /// The `loco-rs` version requirement written into every generated app must
    /// name the release this CLI generates for.
    ///
    /// A floor below the current release resolves to an older published
    /// `loco-rs` that cannot read the templates this CLI emits — a generated
    /// app that compiles and then dies on startup. Every generator test hides
    /// this by setting `LOCO_DEV_MODE_PATH`, so this is the only check on it.
    #[test]
    fn loco_version_floor_tracks_the_framework_release() {
        let manifest_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../Cargo.toml");
        let manifest: toml::Value = toml::from_str(
            &std::fs::read_to_string(manifest_path).expect("read the loco-rs manifest"),
        )
        .expect("parse the loco-rs manifest");

        assert_eq!(
            manifest["package"]["name"].as_str(),
            Some("loco-rs"),
            "{manifest_path} is not the loco-rs manifest anymore; this test reads the \
             wrong version"
        );

        let framework = semver::Version::parse(
            manifest["package"]["version"]
                .as_str()
                .expect("loco-rs package.version"),
        )
        .expect("loco-rs version is semver");

        assert_eq!(
            LOCO_VERSION,
            format!("{}.{}", framework.major, framework.minor),
            "LOCO_VERSION is the `loco-rs` requirement in every generated Cargo.toml, and \
             loco-rs is now {framework}. A stale floor lets cargo resolve an older \
             published loco-rs that cannot parse the config this CLI generates, so the \
             app compiles and fails to boot. Fix it in loco-new/src/lib.rs (or run \
             `cargo xtask bump`, which maintains it)."
        );

        assert!(
            semver::VersionReq::parse(LOCO_VERSION)
                .expect("LOCO_VERSION parses as a cargo version requirement")
                .matches(&framework),
            "LOCO_VERSION ({LOCO_VERSION}) excludes the loco-rs release it ships with \
             ({framework})"
        );
    }
}
