//! # Task Management Module
//!
//! This module defines the task management framework used to manage and execute
//! tasks in a web server application.
use std::collections::BTreeMap;

use async_trait::async_trait;

use crate::{app::AppContext, errors::Error, Result};

/// Struct representing a collection of task arguments.
#[derive(Default, Debug)]
pub struct Vars {
    /// A list of cli arguments.
    pub cli: BTreeMap<String, String>,
}

impl Vars {
    /// Adds a new key-value pair to the `cli` BTreeMap.
    ///
    /// # Arguments
    ///
    /// * `key` - A string representing the key.
    /// * `value` - A string representing the value.
    ///
    /// # Example
    ///
    /// ```
    /// use loco_rs::task::Vars;
    ///
    /// let mut vars = Vars::default();
    /// vars.add_cli_arg("key1".to_string(), "value1".to_string());
    /// ```
    pub fn add_cli_arg(&mut self, key: String, value: String) {
        self.cli.insert(key, value);
    }

    /// Retrieves the value associated with the given key from the `cli`
    /// BTreeMap.
    ///
    /// # Errors
    ///
    /// Returns an error if the key does not exist.
    ///
    /// # Example
    ///
    /// ```
    /// use loco_rs::task::Vars;
    ///
    /// let mut vars = Vars::default();
    /// vars.add_cli_arg("key1".to_string(), "value1".to_string());
    ///
    /// assert!(vars.cli_arg("key1").is_ok());
    /// assert!(vars.cli_arg("not-exists").is_err());
    /// ```
    pub fn cli_arg(&self, key: &str) -> Result<&String> {
        Ok(self
            .cli
            .get(key)
            .ok_or(Error::Message(format!("The argument {key} does not exist")))?)
    }
}

/// Information about a task, including its name and details.
#[allow(clippy::module_name_repetitions)]
pub struct TaskInfo {
    pub name: String,
    pub detail: String,
}

/// A trait defining the behavior of a task.
#[async_trait]
pub trait Task: Send + Sync {
    /// Get information about the task.
    fn task(&self) -> TaskInfo;
    /// Execute the task with the provided application context and variables.
    async fn run(&self, app_context: &AppContext, vars: &Vars) -> Result<()>;
}

/// Managing and running tasks.
#[derive(Default)]
pub struct Tasks {
    registry: BTreeMap<String, Box<dyn Task>>,
}

impl Tasks {
    /// List all registered tasks with their information.
    #[must_use]
    pub fn list(&self) -> Vec<TaskInfo> {
        self.registry.values().map(|t| t.task()).collect::<Vec<_>>()
    }

    /// Run a registered task by name with provided variables.
    ///
    /// # Errors
    ///
    /// Returns a [`Result`] if an task finished with error. mostly if the given
    /// task is not found or an error to run the task.s
    pub async fn run(&self, app_context: &AppContext, task: &str, vars: &Vars) -> Result<()> {
        let task = self
            .registry
            .get(task)
            .ok_or_else(|| Error::TaskNotFound(task.to_string()))?;
        task.run(app_context, vars).await?;
        Ok(())
    }

    /// Register a new task to the registry.
    pub fn register(&mut self, task: impl Task + 'static) {
        let name = task.task().name;
        self.registry.insert(name, Box::new(task));
    }
}
