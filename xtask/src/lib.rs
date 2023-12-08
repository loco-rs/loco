use std::process::exit;
pub mod bump_version;
pub mod ci;
pub mod errors;
pub mod out;
pub mod prompt;
pub mod utils;

#[derive(Debug)]
pub struct CmdExit {
    pub code: i32,
    pub message: Option<String>,
}

impl CmdExit {
    #[must_use]
    pub fn error_with_message(message: &str) -> Self {
        Self {
            code: 1,
            message: Some(format!("🙀 {message}")),
        }
    }

    #[must_use]
    pub fn ok_with_message(message: &str) -> Self {
        Self {
            code: 0,
            message: Some(message.to_string()),
        }
    }

    #[must_use]
    pub const fn ok() -> Self {
        Self {
            code: 0,
            message: None,
        }
    }

    pub fn exit(&self) {
        if let Some(message) = &self.message {
            eprintln!("{message}");
        };

        exit(self.code);
    }
}
