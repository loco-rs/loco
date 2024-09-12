//! # Scheduler Module
//! TBD

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt, io,
    path::{Path, PathBuf},
    time::Instant,
};

use crate::{app::Hooks, environment::Environment, task::Tasks};

use tokio_cron_scheduler::{JobScheduler, JobSchedulerError};

lazy_static::lazy_static! {
    static ref RE_IS_CRON_SYNTAX: Regex = Regex::new(r"^[\*\d]").unwrap();
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
    /// The cron expression defining the job's schedule.
    ///
    /// The format is as follows:
    /// sec   min   hour   day of month   month   day of week   year
    /// *     *     *      *              *       *             *
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
            "#      job_name        cron               tags               run"
        )?;

        let mut job_names: Vec<&String> = self.jobs.keys().collect();
        job_names.sort();

        for (index, &job_name) in job_names.iter().enumerate() {
            if let Some(job) = self.jobs.get(job_name) {
                writeln!(
                    f,
                    "{:<6} {:<15} {:<18} {:<18} {:?}",
                    index + 1,
                    job_name,
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
#[derive(Clone)]
pub struct Scheduler {
    pub jobs: HashMap<String, Job>,
    binary_path: PathBuf,
    default_output: Output,
    environment: Environment,
}

/// Specification used to filter all scheduler job with the given Spec.
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
        tracing::info!(command = &self.command, "execute jon command");
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
    /// When creating a new scheduler instance all register task should be loaded for validate the
    /// given configuration.
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

            let cron_syntax = if RE_IS_CRON_SYNTAX.is_match(&job.cron) {
                job.cron.clone()
            } else {
                english_to_cron::str_cron_syntax(&job.cron).map_err(|err| {
                    Error::InvalidCronSyntax {
                        cron: job.cron.clone(),
                        error: err.to_string(),
                    }
                })?
            };

            let job_name = job_name.to_string();
            sched
                .add(tokio_cron_scheduler::Job::new_async(
                    cron_syntax.as_str(),
                    move |uuid, mut _l| {
                        let job_description = job_description.clone();
                        let job_name = job_name.to_string();
                        Box::pin(async move {
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
                            };
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

#[cfg(test)]
mod tests {

    use super::*;
    use crate::tests_cfg;
    use insta::assert_debug_snapshot;

    use rstest::rstest;
    use tests_cfg::db::AppHook;
    use tokio::time::{self, Duration};

    fn get_scheduler_from_config() -> Result<Scheduler, Error> {
        let scheduler_config_path = PathBuf::from("tests")
            .join("fixtures")
            .join("scheduler")
            .join("scheduler.yaml");

        Scheduler::from_config::<AppHook>(&scheduler_config_path, &Environment::Development)
    }

    #[test]
    pub fn can_display_scheduler() {
        let scheduler = get_scheduler_from_config().unwrap();
        assert_debug_snapshot!(format!("{scheduler}"));
    }

    #[test]
    pub fn can_load_from_config_local_config() {
        assert!(get_scheduler_from_config().is_ok());
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
        let scheduler = get_scheduler_from_config().unwrap().by_spec(&Spec {
            name: None,
            tag: Some("base".to_string()),
        });

        assert_eq!(scheduler.jobs.len(), 2);
    }

    #[test]
    pub fn can_load_jobs_by_spec_tag_single_jobs() {
        let scheduler = get_scheduler_from_config().unwrap().by_spec(&Spec {
            name: None,
            tag: Some("echo".to_string()),
        });

        assert_eq!(scheduler.jobs.len(), 1);
        assert!(scheduler.jobs.contains_key("print_task"));
    }

    #[test]
    pub fn can_load_jobs_by_spec_with_job_name() {
        let scheduler = get_scheduler_from_config().unwrap().by_spec(&Spec {
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
        let mut scheduler = get_scheduler_from_config().unwrap();

        let path = tree_fs::Tree::default()
            .add("scheduler.txt", "")
            .add("scheduler2.txt", "")
            .create()
            .unwrap();

        assert_eq!(
            std::fs::read_to_string(&path.join("scheduler.txt"))
                .unwrap()
                .lines()
                .count(),
            0
        );
        assert_eq!(
            std::fs::read_to_string(&path.join("scheduler2.txt"))
                .unwrap()
                .lines()
                .count(),
            0
        );

        scheduler.jobs = HashMap::from([
            (
                "test".to_string(),
                Job {
                    run: format!("echo loco >> {}", path.join("scheduler.txt").display()),
                    shell: true,
                    cron: "run every 1 second".to_string(),
                    tags: None,
                    output: None,
                },
            ),
            (
                "test_2".to_string(),
                Job {
                    run: format!("echo loco >> {}", path.join("scheduler2.txt").display()),
                    shell: true,
                    cron: "* * * * * ? *".to_string(),
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
            std::fs::read_to_string(path.join("scheduler.txt"))
                .unwrap()
                .lines()
                .count()
                >= 4
        );
        assert!(
            std::fs::read_to_string(path.join("scheduler2.txt"))
                .unwrap()
                .lines()
                .count()
                >= 4
        );
    }
}
