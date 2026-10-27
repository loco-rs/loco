//! # Task Management Module
//!
//! This module defines the task management framework used to manage and execute
//! tasks in a web server application.
use std::collections::BTreeMap;

use async_trait::async_trait;

use crate::{app::AppContext, errors::Error, Result};

/// Struct representing a collection of task arguments.
///
/// A task sees its command line two ways. Pick one per task:
///
/// * [`Vars::cli_arg`] / [`Vars::cli`]: the `key:value` tokens, parsed for you.
/// * [`Vars::args`]: every token, verbatim and in order. Feed it to `clap`, or
///   use it as a file list (`cargo loco task import *.csv`).
///
/// A token such as `C:\data.csv` contains a colon, so it shows up in `cli` as
/// well as in `args()`. Tasks that take free-form arguments should read only
/// `args()`.
#[derive(Default, Debug)]
#[non_exhaustive]
pub struct Vars {
    /// The `key:value` cli arguments.
    pub cli: BTreeMap<String, String>,
    args: Vec<String>,
}

impl Vars {
    /// Create [`Vars`] instance from cli arguments.
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
    /// let args = vec![("key1".to_string(), "value".to_string())];
    /// let vars = Vars::from_cli_args(args);
    /// ```
    #[must_use]
    pub fn from_cli_args(args: Vec<(String, String)>) -> Self {
        Self {
            args: args.iter().map(|(k, v)| format!("{k}:{v}")).collect(),
            cli: args.into_iter().collect(),
        }
    }

    /// Create [`Vars`] from the raw tokens given after the task name.
    ///
    /// Every token is kept verbatim for [`Vars::args`]. Tokens of the form
    /// `key:value` are additionally available through [`Vars::cli_arg`];
    /// anything else (a glob result, a flag, a path) is not an error.
    ///
    /// # Example
    ///
    /// ```
    /// use loco_rs::task::Vars;
    ///
    /// let vars = Vars::from_args(vec!["a.txt".to_string(), "mode:fast".to_string()]);
    /// assert_eq!(vars.args(), ["a.txt", "mode:fast"]);
    /// assert_eq!(vars.cli_arg("mode").unwrap(), "fast");
    /// ```
    #[must_use]
    pub fn from_args(args: Vec<String>) -> Self {
        let cli = args
            .iter()
            .filter_map(|a| a.split_once(':'))
            .filter(|(k, _)| !k.is_empty())
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Self { cli, args }
    }

    /// Every argument given after the task name, verbatim and in order.
    ///
    /// # Example
    ///
    /// ```
    /// use loco_rs::task::Vars;
    ///
    /// let vars = Vars::from_args(vec!["--dry-run".to_string(), "a.txt".to_string()]);
    /// assert_eq!(vars.args(), ["--dry-run", "a.txt"]);
    /// ```
    #[must_use]
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// Retrieves the value associated with the given key from the `cli` list.
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
    /// let args = vec![("key1".to_string(), "value".to_string())];
    /// let vars = Vars::from_cli_args(args);
    ///
    /// assert!(vars.cli_arg("key1").is_ok());
    /// assert!(vars.cli_arg("not-exists").is_err());
    /// ```
    pub fn cli_arg(&self, key: &str) -> Result<&str> {
        self.cli
            .get(key)
            .map(String::as_str)
            .ok_or(Error::Message(format!("the argument {key} does not exist")))
    }
}

/// Information about a task, including its name and details.
#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
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

    /// List of all tasks names
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        self.registry
            .values()
            .map(|t| t.task().name)
            .collect::<Vec<_>>()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests_cfg;

    #[tokio::test]
    async fn test_vars_from_cli_args() {
        let args = vec![
            ("key1".to_string(), "value1".to_string()),
            ("key2".to_string(), "value2".to_string()),
        ];
        let vars = Vars::from_cli_args(args);

        assert_eq!(vars.cli.len(), 2);
        assert_eq!(vars.cli.get("key1"), Some(&"value1".to_string()));
        assert_eq!(vars.cli.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn from_args_is_lenient_and_keeps_everything() {
        let vars = Vars::from_args(
            ["a.txt", "--force", "k:v", "url:http://x:80", ":x"]
                .map(String::from)
                .to_vec(),
        );
        assert_eq!(vars.args().len(), 5);
        assert_eq!(vars.args()[1], "--force");
        assert_eq!(vars.cli.len(), 2);
        assert_eq!(vars.cli_arg("k").unwrap(), "v");
        assert_eq!(vars.cli_arg("url").unwrap(), "http://x:80");
        assert!(vars.cli_arg("a.txt").is_err());
    }

    #[test]
    fn from_cli_args_populates_args() {
        let vars = Vars::from_cli_args(vec![("k".into(), "v".into())]);
        assert_eq!(vars.args(), ["k:v"]);
    }

    #[tokio::test]
    async fn test_vars_cli_arg() {
        let args = vec![("key1".to_string(), "value1".to_string())];
        let vars = Vars::from_cli_args(args);

        assert_eq!(vars.cli_arg("key1").unwrap(), "value1");
        assert!(vars.cli_arg("not-exists").is_err());
    }

    #[tokio::test]
    async fn test_tasks_registry() {
        let mut tasks = Tasks::default();
        tasks.register(tests_cfg::task::Foo);
        tasks.register(tests_cfg::task::ParseArgs);

        assert_eq!(tasks.names().len(), 2);
        assert!(tasks.names().contains(&"foo".to_string()));
        assert!(tasks.names().contains(&"parse_args".to_string()));
    }

    #[tokio::test]
    async fn test_tasks_list() {
        let mut tasks = Tasks::default();
        tasks.register(tests_cfg::task::Foo);
        tasks.register(tests_cfg::task::ParseArgs);

        let task_infos = tasks.list();
        assert_eq!(task_infos.len(), 2);

        let names: Vec<String> = task_infos.iter().map(|info| info.name.clone()).collect();
        let details: Vec<String> = task_infos.iter().map(|info| info.detail.clone()).collect();

        assert!(names.contains(&"foo".to_string()));
        assert!(names.contains(&"parse_args".to_string()));
        assert!(details.contains(&"run foo task".to_string()));
        assert!(details.contains(&"Validate the paring args".to_string()));
    }

    #[tokio::test]
    async fn test_tasks_run_success() {
        let mut tasks = Tasks::default();
        tasks.register(tests_cfg::task::Foo);

        let app_context = tests_cfg::app::get_app_context().await;
        let vars = Vars::default();

        let result = tasks.run(&app_context, "foo", &vars).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_tasks_run_failure() {
        let mut tasks = Tasks::default();
        tasks.register(tests_cfg::task::ParseArgs);

        let app_context = tests_cfg::app::get_app_context().await;
        let vars = Vars::default();

        // ParseArgs will fail with "invalid args" if app != "loco" or refresh != true
        let result = tasks.run(&app_context, "parse_args", &vars).await;
        assert!(result.is_err());

        if let Err(Error::Message(msg)) = result {
            assert_eq!(msg, "invalid args");
        } else {
            panic!("Expected Error::Message variant");
        }
    }

    #[tokio::test]
    async fn test_tasks_run_with_args() {
        let mut tasks = Tasks::default();
        tasks.register(tests_cfg::task::ParseArgs);

        let app_context = tests_cfg::app::get_app_context().await;
        let args = vec![
            ("test".to_string(), "true".to_string()),
            ("app".to_string(), "loco".to_string()),
        ];
        let vars = Vars::from_cli_args(args);

        // ParseArgs will succeed when app == "loco" and test == "true"
        let result = tasks.run(&app_context, "parse_args", &vars).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_tasks_run_not_found() {
        let tasks = Tasks::default();
        let app_context = tests_cfg::app::get_app_context().await;
        let vars = Vars::default();

        let result = tasks.run(&app_context, "non_existent_task", &vars).await;
        assert!(result.is_err());

        match result {
            Err(Error::TaskNotFound(task_name)) => {
                assert_eq!(task_name, "non_existent_task");
            }
            _ => panic!("Expected Error::TaskNotFound variant"),
        }
    }

    #[tokio::test]
    async fn test_task_registration_and_override() {
        // Create a custom task that will override Foo
        struct CustomFoo;

        #[async_trait]
        impl Task for CustomFoo {
            fn task(&self) -> TaskInfo {
                TaskInfo {
                    name: "foo".to_string(),
                    detail: "Updated foo task".to_string(),
                }
            }

            async fn run(&self, _app_context: &AppContext, _vars: &Vars) -> Result<()> {
                Ok(())
            }
        }

        let mut tasks = Tasks::default();
        tasks.register(tests_cfg::task::Foo);
        assert_eq!(tasks.names().len(), 1);

        // Register a new task with the same name
        tasks.register(CustomFoo);

        // Should still have only one task (overwritten)
        assert_eq!(tasks.names().len(), 1);

        let task_infos = tasks.list();
        assert_eq!(task_infos[0].detail, "Updated foo task");
    }
}
