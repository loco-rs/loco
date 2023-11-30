#![allow(clippy::missing_errors_doc)]

use std::process::exit;
pub mod generate;
pub mod template;

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
            message: Some(message.to_string()),
        }
    }
    #[must_use]
    pub fn ok_with_message(message: &str) -> Self {
        Self {
            code: 0,
            message: Some(message.to_string()),
        }
    }

    pub fn exit(&self) {
        if let Some(message) = &self.message {
            eprintln!("{message}");
        };

        exit(self.code);
    }
}
