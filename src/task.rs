//! # Task Management Module
//!
//! This module defines the task management framework used to manage and execute
//! tasks in a web server application.
use std::{collections::BTreeMap, future::Future};

use async_trait::async_trait;

use crate::{app::AppContext, errors::Error, Result};

/// Parsed key:value arguments for tasks that prefer structured args.
///
/// This is a helper for tasks that want the `key:value` argument style.
///
/// # Example
///
/// ```
/// use loco_rs::task::Vars;
///
/// let args = vec!["key1:value1".to_string(), "key2:value2".to_string()];
/// let vars = Vars::from_args(&args);
/// assert_eq!(vars.cli_arg("key1"), Some(&"value1".to_string()));
/// ```
#[derive(Default, Debug)]
pub struct Vars {
    /// A map of parsed key:value arguments.
    pub cli: BTreeMap<String, String>,
}

impl Vars {
    /// Parse arguments in `key:value` format.
    ///
    /// Arguments not matching the format are ignored.
    ///
    /// # Example
    ///
    /// ```
    /// use loco_rs::task::Vars;
    ///
    /// let args = vec!["name:John".to_string(), "age:30".to_string(), "plain_arg".to_string()];
    /// let vars = Vars::from_args(&args);
    /// assert_eq!(vars.cli_arg("name"), Some(&"John".to_string()));
    /// assert_eq!(vars.cli_arg("age"), Some(&"30".to_string()));
    /// assert_eq!(vars.cli_arg("plain_arg"), None); // Not in key:value format
    /// ```
    #[must_use]
    pub fn from_args(args: &[String]) -> Self {
        let cli = args
            .iter()
            .filter_map(|s| {
                let pos = s.find(':')?;
                Some((s[..pos].to_string(), s[pos + 1..].to_string()))
            })
            .collect();
        Self { cli }
    }

    /// Get a CLI argument by key.
    ///
    /// Returns `Some` if the key exists, `None` otherwise.
    #[must_use]
    pub fn cli_arg(&self, key: &str) -> Option<&String> {
        self.cli.get(key)
    }

    /// Get a CLI argument by key, returning an error if missing.
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
    /// let args = vec!["key1:value".to_string()];
    /// let vars = Vars::from_args(&args);
    ///
    /// assert!(vars.require("key1").is_ok());
    /// assert!(vars.require("not-exists").is_err());
    /// ```
    pub fn require(&self, key: &str) -> Result<&String> {
        self.cli
            .get(key)
            .ok_or_else(|| Error::Message(format!("required argument '{key}' not provided")))
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
///
/// Tasks receive raw command-line arguments as a slice of strings,
/// allowing full flexibility in argument parsing.
///
/// # Example
///
/// ```rust,ignore
/// use loco_rs::prelude::*;
///
/// pub struct MyTask;
///
/// #[async_trait]
/// impl Task for MyTask {
///     fn task(&self) -> TaskInfo {
///         TaskInfo {
///             name: "my_task".to_string(),
///             detail: "Does something useful".to_string(),
///         }
///     }
///
///     async fn run(&self, ctx: &AppContext, args: &[String]) -> Result<()> {
///         for arg in args {
///             println!("Processing: {}", arg);
///         }
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait Task: Send + Sync {
    /// Get information about the task.
    fn task(&self) -> TaskInfo;
    /// Execute the task with the provided application context and arguments.
    async fn run(&self, app_context: &AppContext, args: &[String]) -> Result<()>;
}

/// A trait for tasks that want to use the `key:value` argument parsing.
///
/// Implement this trait and wrap with [`VarsTask`] to get automatic
/// parsing of `key:value` arguments into a [`Vars`] struct.
///
/// # Example
///
/// ```rust,ignore
/// use loco_rs::prelude::*;
/// use loco_rs::task::{VarsTask, VarsTaskHandler, Vars};
///
/// pub struct MyTask;
///
/// impl VarsTaskHandler for MyTask {
///     fn task(&self) -> TaskInfo {
///         TaskInfo {
///             name: "my_task".to_string(),
///             detail: "Create something: name:<NAME> count:<COUNT>".to_string(),
///         }
///     }
///
///     async fn run(&self, ctx: &AppContext, vars: &Vars) -> Result<()> {
///         let name = vars.require("name")?;
///         let count = vars.cli_arg("count").map(|s| s.parse::<i32>().unwrap_or(1));
///         // ... do something
///         Ok(())
///     }
/// }
///
/// // Register with: tasks.register(VarsTask(MyTask));
/// // Usage: cargo loco task my_task name:foo count:5
/// ```
pub trait VarsTaskHandler: Send + Sync {
    /// Get information about the task.
    fn task(&self) -> TaskInfo;
    /// Execute the task with parsed key:value arguments.
    fn run(
        &self,
        app_context: &AppContext,
        vars: &Vars,
    ) -> impl Future<Output = Result<()>> + Send;
}

/// A wrapper that parses `key:value` args into [`Vars`] for the inner task.
///
/// Use this to wrap a [`VarsTaskHandler`] implementation so it can be
/// registered as a [`Task`].
///
/// # Example
///
/// ```rust,ignore
/// tasks.register(VarsTask(MyVarsTask));
/// ```
pub struct VarsTask<T: VarsTaskHandler>(pub T);

#[async_trait]
impl<T: VarsTaskHandler + 'static> Task for VarsTask<T> {
    fn task(&self) -> TaskInfo {
        self.0.task()
    }

    async fn run(&self, app_context: &AppContext, args: &[String]) -> Result<()> {
        let vars = Vars::from_args(args);
        self.0.run(app_context, &vars).await
    }
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

    /// Run a registered task by name with provided arguments.
    ///
    /// # Errors
    ///
    /// Returns a [`Result`] if a task finished with error. Mostly if the given
    /// task is not found or an error running the task.
    pub async fn run(&self, app_context: &AppContext, task: &str, args: &[String]) -> Result<()> {
        let task = self
            .registry
            .get(task)
            .ok_or_else(|| Error::TaskNotFound(task.to_string()))?;
        task.run(app_context, args).await?;
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
    async fn test_vars_from_args() {
        let args = vec![
            "key1:value1".to_string(),
            "key2:value2".to_string(),
            "plain_arg".to_string(),
        ];
        let vars = Vars::from_args(&args);

        assert_eq!(vars.cli.len(), 2);
        assert_eq!(vars.cli.get("key1"), Some(&"value1".to_string()));
        assert_eq!(vars.cli.get("key2"), Some(&"value2".to_string()));
        assert_eq!(vars.cli.get("plain_arg"), None);
    }

    #[tokio::test]
    async fn test_vars_cli_arg() {
        let args = vec!["key1:value1".to_string()];
        let vars = Vars::from_args(&args);

        assert_eq!(vars.cli_arg("key1"), Some(&"value1".to_string()));
        assert_eq!(vars.cli_arg("not-exists"), None);
    }

    #[tokio::test]
    async fn test_vars_require() {
        let args = vec!["key1:value1".to_string()];
        let vars = Vars::from_args(&args);

        assert_eq!(vars.require("key1").unwrap(), "value1");
        assert!(vars.require("not-exists").is_err());
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
        let args: Vec<String> = vec![];

        let result = tasks.run(&app_context, "foo", &args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_tasks_run_failure() {
        let mut tasks = Tasks::default();
        tasks.register(tests_cfg::task::ParseArgs);

        let app_context = tests_cfg::app::get_app_context().await;
        let args: Vec<String> = vec![];

        // ParseArgs will fail with "invalid args" if app != "loco" or refresh != true
        let result = tasks.run(&app_context, "parse_args", &args).await;
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
        let args = vec!["test:true".to_string(), "app:loco".to_string()];

        // ParseArgs will succeed when app == "loco" and test == "true"
        let result = tasks.run(&app_context, "parse_args", &args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_tasks_run_not_found() {
        let tasks = Tasks::default();
        let app_context = tests_cfg::app::get_app_context().await;
        let args: Vec<String> = vec![];

        let result = tasks.run(&app_context, "non_existent_task", &args).await;
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

            async fn run(&self, _app_context: &AppContext, _args: &[String]) -> Result<()> {
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
