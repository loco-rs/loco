//! # Task Management Module
//!
//! This module defines the task management framework used to manage and execute tasks in a web server application.
//!
use std::collections::BTreeMap;

use async_trait::async_trait;

use crate::{app::AppContext, errors::Error, Result};

/// Information about a task, including its name and details.
#[allow(clippy::module_name_repetitions)]
pub struct TaskInfo {
    pub name: String,
    pub detail: String,
}

/// A trait defining the behavior of a task.
#[async_trait]
pub trait Task {
    /// Get information about the task.
    fn task(&self) -> TaskInfo;
    /// Execute the task with the provided application context and variables.
    async fn run(&self, app_context: &AppContext, vars: &BTreeMap<String, String>) -> Result<()>;
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
    /// Returns a [`Result`] if an task finished with error. mostly if the given task is not found or an error to run the task.s
    pub async fn run(
        &self,
        app_context: &AppContext,
        task: &str,
        vars: &BTreeMap<String, String>,
    ) -> Result<()> {
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
