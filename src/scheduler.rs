//! # Scheduler Module
//! TBD

use std::{
    collections::HashMap,
    fmt, io,
    path::{Path, PathBuf},
    sync::OnceLock,
    time::{Duration, Instant},
};

use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio_cron_scheduler::{JobScheduler, JobSchedulerError};
use uuid::Uuid;

use crate::{app::Hooks, environment::Environment, task::Tasks};

static RE_IS_CRON_SYNTAX: OnceLock<Regex> = OnceLock::new();

fn get_re_is_cron_syntax() -> &'static Regex {
    RE_IS_CRON_SYNTAX.get_or_init(|| Regex::new(r"^[\*\d]").unwrap())
}

/// Errors that may occur while operating the scheduler.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("schedulers not configured")]
    Empty,

    #[error("task `{0}` not found")]
    TaskNotFound(String),

    #[error("Scheduler config file not found in path: '{}'", path.display())]
    ConfigNotFound { path: PathBuf, error: io::Error },

    #[error("Invalid scheduler config schema. err: '{}'", error.as_display())]
    InvalidConfigSchema { error: serde_yaml::Error },

    #[error("Invalid cron {cron}. err: '{}'", error.as_display())]
    InvalidCronSyntax { cron: String, error: String },

    #[error(transparent)]
    Question(#[from] JobSchedulerError),

    #[error(transparent)]
    IO(#[from] std::io::Error),
}

/// Result type used in the module, with a custom error type.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Configuration structure for the scheduler.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// A list of jobs to be scheduled.
    pub jobs: HashMap<String, Job>,
    /// The default output setting for the jobs.
    #[serde(default)]
    pub output: Output,
}

/// Representing a single job in the scheduler.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Job {
    /// The command to run.
    /// In case of task: it should be a task name and also task arguments
    pub run: String,
    #[serde(default)]
    pub shell: bool,
    #[serde(default)]
    pub run_on_start: bool,
    #[serde(rename = "schedule")]
    /// The cron expression defining the job's schedule.
    ///
    /// The format is as follows:
    /// sec   min   hour   day of month   month   day of week   year
    /// * *     *      *              *       *             *
    pub cron: String,
    /// Tags for tagging the job.
    pub tags: Option<Vec<String>>,
    /// Output settings for the job.
    pub output: Option<Output>,
}

impl fmt::Display for Scheduler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "#      job_name       run_on_start      schedule               tags               run"
        )?;

        let mut job_names: Vec<&String> = self.jobs.keys().collect();
        job_names.sort();

        for (index, &job_name) in job_names.iter().enumerate() {
            if let Some(job) = self.jobs.get(job_name) {
                writeln!(
                    f,
                    "{:<6} {:<15} {:<12} {:<22} {:<18} {:?}",
                    index + 1,
                    job_name,
                    job.run_on_start,
                    job.cron,
                    job.tags
                        .as_ref()
                        .map_or("-".to_string(), |tags| tags.join(", ")),
                    job.run,
                )?;
            }
        }

        Ok(())
    }
}

/// Representing the scheduler itself.
#[derive(Clone, Debug)]
pub struct Scheduler {
    pub jobs: HashMap<String, Job>,
    binary_path: PathBuf,
    default_output: Output,
    environment: Environment,
}

/// Specification used to filter all scheduler job with the given Spec.
#[derive(Debug)]
pub struct Spec {
    pub name: Option<String>,
    pub tag: Option<String>,
}

/// Enum representing the scheduler job output.
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub enum Output {
    /// Silent output, the STDOUT or STDERR of the job will not view out.
    #[serde(rename = "silent")]
    Silent,
    /// The STDOUT or STDERR of the job will view propagated
    #[default]
    #[serde(rename = "stdout")]
    STDOUT,
}

/// Structure representing the job command.
#[derive(Clone, Debug)]
pub struct JobDescription {
    /// The command to execute.
    pub command: String,
    /// The output setting for the job.
    pub output: Output,
    /// The environment in which the job will run.
    pub environment: Environment,
}

impl Job {
    /// Prepares the command for execution based on the job's configuration.
    #[must_use]
    pub fn prepare_command(
        &self,
        binary_path: &Path,
        default_output: &Output,
        environment: &Environment,
    ) -> JobDescription {
        let command = if self.shell {
            self.run.to_string()
        } else {
            [
                binary_path.display().to_string(),
                "task".to_string(),
                self.run.to_string(),
            ]
            .join(" ")
        };

        JobDescription {
            command,
            output: self
                .output
                .clone()
                .unwrap_or_else(|| default_output.clone()),
            environment: environment.clone(),
        }
    }
}

impl JobDescription {
    /// Executes the job command and returns the output.
    ///
    /// # Errors
    ///
    /// In addition to all the IO errors possible
    pub fn run(&self) -> io::Result<std::process::Output> {
        tracing::info!(command = &self.command, "execute job command");
        let mut exec_job =
            duct_sh::sh_dangerous(&self.command).env("LOCO_ENV", self.environment.to_string());
        exec_job = match self.output {
            Output::Silent => exec_job.stdout_null().stderr_null(),
            Output::STDOUT => exec_job,
        };

        exec_job.run()
    }
}

impl Scheduler {
    /// Creates a new scheduler instance from the given configuration file.
    ///
    /// # Errors
    ///
    /// When could not parse the given file content into a [`Config`] struct.
    pub fn from_config<H: Hooks>(config: &Path, environment: &Environment) -> Result<Self> {
        let config_str =
            std::fs::read_to_string(config).map_err(|error| Error::ConfigNotFound {
                path: config.to_path_buf(),
                error,
            })?;

        let config: Config = serde_yaml::from_str(&config_str)
            .map_err(|error| Error::InvalidConfigSchema { error })?;

        Self::new::<H>(&config, environment)
    }

    /// Creates a new scheduler instance from the provided configuration data.
    ///
    /// When creating a new scheduler instance all register task should be
    /// loaded for validate the given configuration.
    ///
    /// # Errors
    ///
    /// When there is not job in the given config
    pub fn new<H: Hooks>(data: &Config, environment: &Environment) -> Result<Self> {
        let mut tasks = Tasks::default();
        H::register_tasks(&mut tasks);

        let mut jobs = HashMap::new();
        for (job_name, job) in &data.jobs {
            if job.shell {
                jobs.insert(job_name.to_string(), job.clone());
            } else {
                let task_name = job.run.split_whitespace().next().unwrap_or("");
                if tasks.names().iter().any(|name| name.as_str() == task_name) {
                    jobs.insert(job_name.to_string(), job.clone());
                } else {
                    return Err(Error::TaskNotFound(task_name.to_string()));
                }
            }
        }

        if jobs.is_empty() {
            return Err(Error::Empty);
        }

        Ok(Self {
            jobs,
            binary_path: std::env::current_exe()?,
            default_output: data.output.clone(),
            environment: environment.clone(),
        })
    }

    /// Filters the scheduler's jobs based on the provided specification.
    #[must_use]
    pub fn by_spec(self, include_jobs: &Spec) -> Self {
        let jobs = self
            .jobs
            .into_iter()
            .filter(|(job_name, job)| {
                if let Some(name) = &include_jobs.name {
                    return name == job_name;
                }

                if let Some(tag) = &include_jobs.tag {
                    if let Some(job_tags) = &job.tags {
                        return job_tags.contains(tag);
                    }
                }

                true
            })
            .collect::<HashMap<String, Job>>();

        Self { jobs, ..self }
    }

    /// Runs the scheduled jobs according to their cron expressions.
    ///
    /// # Errors
    ///
    /// When could not add job to the scheduler
    pub async fn run(self) -> Result<()> {
        let mut sched = JobScheduler::new().await?;

        for (job_name, job) in &self.jobs {
            let job_description =
                job.prepare_command(&self.binary_path, &self.default_output, &self.environment);

            let cron_syntax = if get_re_is_cron_syntax().is_match(&job.cron) {
                job.cron.clone()
            } else {
                english_to_cron::str_cron_syntax(&job.cron).map_err(|err| {
                    Error::InvalidCronSyntax {
                        cron: job.cron.clone(),
                        error: err.to_string(),
                    }
                })?
            };

            if job.run_on_start {
                let job_description = job_description.clone();
                let job_name = job_name.to_string();
                sched
                    .add(tokio_cron_scheduler::Job::new_one_shot_async(
                        Duration::from_secs(0),
                        move |uuid, _l| {
                            let job_description = job_description.clone();
                            let job_name = job_name.clone();
                            Box::pin(async move {
                                execute_job(job_name.as_str(), uuid, &job_description);
                            })
                        },
                    )?)
                    .await?;
            }

            let job_name = job_name.to_string();
            sched
                .add(tokio_cron_scheduler::Job::new_async(
                    cron_syntax.as_str(),
                    move |uuid, mut _l| {
                        let job_description = job_description.clone();
                        let job_name = job_name.to_string();
                        Box::pin(async move {
                            execute_job(job_name.as_str(), uuid, &job_description);
                        })
                    },
                )?)
                .await?;
        }

        sched.start().await?;

        tokio::signal::ctrl_c().await?;
        sched.shutdown().await?;

        Ok(())
    }
}

fn execute_job(job_name: &str, uuid: Uuid, job_description: &JobDescription) {
    let task_span = tracing::span!(
        tracing::Level::DEBUG,
        "run_job",
        job_name,
        job_id = ?uuid,
    );
    let start = Instant::now();
    let _guard = task_span.enter();
    match job_description.run() {
        Ok(output) => {
            tracing::debug!(
                duration = ?start.elapsed(),
                status_code = output.status.code(),
                "execute scheduler job finished"
            );
        }
        Err(err) => {
            tracing::error!(
                duration = ?start.elapsed(),
                error = %err,
                "failed to execute scheduler job in sub process"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;
    use rstest::rstest;
    use tests_cfg::db::AppHook;
    use tokio::time::{self, Duration};
    use tree_fs::TreeBuilder;

    use super::*;
    use crate::tests_cfg;

    fn setup_scheduler_config() -> (Scheduler, tree_fs::Tree) {
        let tree = TreeBuilder::default()
            .add_file(
                "scheduler.yaml",
                r#"
jobs:
  print_task:
    run: foo
    schedule: "*/5 * * * * *"
    tags:
      - base
      - echo

  write_to_file:
    run: "echo loco >> ./scheduler.txt"
    shell: true
    schedule: "*/5 * * * * *"
    tags:
      - base
      - write

  run_on_start_task:
    run: "echo \"Does this run on start?\" >> ./run_on_start.txt "
    shell: true
    schedule: "every 24 hours"
    run_on_start: true
    tags:
      - start
"#,
            )
            .create()
            .expect("Failed to create test directory structure");

        let scheduler = Scheduler::from_config::<AppHook>(
            &tree.root.join("scheduler.yaml"),
            &Environment::Development,
        )
        .expect("Failed to create scheduler from config");

        (scheduler, tree)
    }

    #[test]
    pub fn can_display_scheduler() {
        let (scheduler, _tree) = setup_scheduler_config();
        assert_debug_snapshot!(format!("{scheduler}"));
    }

    #[test]
    pub fn can_load_from_config_local_config() {
        let (_, _tree) = setup_scheduler_config();
        // If we got here, the setup was successful
        assert!(true);
    }

    #[tokio::test]
    pub async fn can_load_from_env_config() {
        let app_context = tests_cfg::app::get_app_context().await;
        let scheduler = Scheduler::new::<AppHook>(
            &app_context.config.scheduler.unwrap(),
            &Environment::Development,
        );

        assert!(scheduler.is_ok());
    }

    #[test]
    pub fn can_load_jobs_by_spec_tag_multiple_jobs() {
        let (scheduler, _tree) = setup_scheduler_config();
        let scheduler = scheduler.by_spec(&Spec {
            name: None,
            tag: Some("base".to_string()),
        });

        assert_eq!(scheduler.jobs.len(), 2);
    }

    #[test]
    pub fn can_load_jobs_by_spec_tag_single_jobs() {
        let (scheduler, _tree) = setup_scheduler_config();
        let scheduler = scheduler.by_spec(&Spec {
            name: None,
            tag: Some("echo".to_string()),
        });

        assert_eq!(scheduler.jobs.len(), 1);
        assert!(scheduler.jobs.contains_key("print_task"));
    }

    #[test]
    pub fn can_load_jobs_by_spec_with_job_name() {
        let (scheduler, _tree) = setup_scheduler_config();
        let scheduler = scheduler.by_spec(&Spec {
            name: Some("write_to_file".to_string()),
            tag: None,
        });

        assert_eq!(scheduler.jobs.len(), 1);
        assert!(scheduler.jobs.contains_key("write_to_file"));
    }

    #[rstest]
    #[case("shell", "echo loco", true)]
    #[case("task", "foo LOCO_ENV:test SCHEDULER:true", false)]
    pub fn can_prepare_command(#[case] test_name: &str, #[case] run: &str, #[case] shell: bool) {
        let job = Job {
            run: run.to_string(),
            shell,
            run_on_start: false,
            cron: "*/5 * * * * *".to_string(),
            tags: None,
            output: None,
        };

        let prepare_command = job.prepare_command(
            PathBuf::from("[BIN_PATH]").as_path(),
            &Output::STDOUT,
            &Environment::Test,
        );
        assert_debug_snapshot!(
            format!("can_prepare_command_[{test_name}]"),
            prepare_command
        );
    }

    #[tokio::test]
    pub async fn can_run() {
        let (mut scheduler, _config_tree) = setup_scheduler_config();

        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .add("scheduler.txt", "")
            .add("scheduler2.txt", "")
            .add("scheduler3.txt", "")
            .create()
            .unwrap();

        assert_eq!(
            std::fs::read_to_string(tree_fs.root.join("scheduler.txt"))
                .unwrap()
                .lines()
                .count(),
            0
        );
        assert_eq!(
            std::fs::read_to_string(tree_fs.root.join("scheduler2.txt"))
                .unwrap()
                .lines()
                .count(),
            0
        );
        assert_eq!(
            std::fs::read_to_string(tree_fs.root.join("scheduler3.txt"))
                .unwrap()
                .lines()
                .count(),
            0
        );

        scheduler.jobs = HashMap::from([
            (
                "test".to_string(),
                Job {
                    run: format!(
                        "echo loco >> {}",
                        tree_fs.root.join("scheduler.txt").display()
                    ),
                    shell: true,
                    run_on_start: false,
                    cron: "run every 1 second".to_string(),
                    tags: None,
                    output: None,
                },
            ),
            (
                "test_2".to_string(),
                Job {
                    run: format!(
                        "echo loco >> {}",
                        tree_fs.root.join("scheduler2.txt").display()
                    ),
                    shell: true,
                    run_on_start: false,
                    cron: "* * * * * ? *".to_string(),
                    tags: None,
                    output: None,
                },
            ),
            (
                "test_3".to_string(),
                Job {
                    run: format!(
                        "echo loco >> {}",
                        tree_fs.root.join("scheduler3.txt").display()
                    ),
                    shell: true,
                    run_on_start: true,
                    cron: "0 0 * * * * *".to_string(),
                    tags: None,
                    output: None,
                },
            ),
        ]);

        let handle = tokio::spawn(async move {
            scheduler.run().await.unwrap();
        });

        time::sleep(Duration::from_secs(5)).await;
        handle.abort();

        assert!(
            std::fs::read_to_string(tree_fs.root.join("scheduler.txt"))
                .unwrap()
                .lines()
                .count()
                >= 4
        );
        assert!(
            std::fs::read_to_string(tree_fs.root.join("scheduler2.txt"))
                .unwrap()
                .lines()
                .count()
                >= 4
        );
        assert_eq!(
            std::fs::read_to_string(tree_fs.root.join("scheduler3.txt"))
                .unwrap()
                .lines()
                .count(),
            1
        );
    }
}
