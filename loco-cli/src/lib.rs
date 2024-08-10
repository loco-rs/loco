use std::process::exit;
mod env_vars;
pub mod generate;
pub mod git;
pub mod prompt;

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

    pub fn exit(&self) {
        if let Some(message) = &self.message {
            eprintln!("{message}");
        };

        exit(self.code);
    }
}
