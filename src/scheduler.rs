//! # Scheduler Module
//! TBD

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::{
    collections::BTreeMap,
    io,
    path::{Path, PathBuf},
    time::Instant,
};

use crate::{app::Hooks, environment::Environment, task::Tasks};

use tokio_cron_scheduler::{JobScheduler, JobSchedulerError};

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
    /// The type of job.
    #[serde(flatten)]
    pub kind: Kind,
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
            "#      job_name        cron               tags               kind"
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
                    job.kind,
                )?;
            }
        }

        Ok(())
    }
}

/// Enum representing the types of jobs that can be scheduled.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Kind {
    /// A job that runs a defined task.
    #[serde(rename = "task")]
    Task {
        name: String,
        vars: Option<BTreeMap<String, String>>,
    },
    /// A job that executes a shell command.
    #[serde(rename = "shell")]
    Shell { command: String },
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
        let command = match &self.kind {
            Kind::Task { name, vars } => {
                let mut result = vec![
                    binary_path.display().to_string(),
                    "task".to_string(),
                    name.to_string(),
                ];
                if let Some(vars) = vars {
                    for (key, value) in vars {
                        result.push(format!("{key}:{value}"));
                    }
                };
                result.join(" ")
            }
            Kind::Shell { command } => command.clone(),
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
            match job.kind {
                Kind::Task { ref name, vars: _ } => {
                    if tasks.names().contains(name) {
                        jobs.insert(job_name.to_string(), job.clone());
                    } else {
                        return Err(Error::TaskNotFound(name.to_string()));
                    }
                }
                Kind::Shell { command: _ } => {
                    jobs.insert(job_name.to_string(), job.clone());
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

            let job_name = job_name.to_string();
            sched
                .add(tokio_cron_scheduler::Job::new_async(
                    job.cron.as_str(),
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
        assert!(scheduler.jobs.contains_key("print task"));
    }

    #[test]
    pub fn can_load_jobs_by_spec_with_job_name() {
        let scheduler = get_scheduler_from_config().unwrap().by_spec(&Spec {
            name: Some("write to file".to_string()),
            tag: None,
        });

        assert_eq!(scheduler.jobs.len(), 1);
        assert!(scheduler.jobs.contains_key("write to file"));
    }

    #[rstest]
    #[case("shell", Kind::Shell {command: "echo loco".to_string()})]
    #[case("task", Kind::Task {name: "foo".to_string(),vars: Some(BTreeMap::from([("LOCO_ENV".to_string(), "test".to_string()),("SCHEDULER".to_string(), "true".to_string())]))})]
    pub fn can_prepare_command(#[case] test_name: &str, #[case] kind: Kind) {
        let job = Job {
            kind,
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
            .create()
            .unwrap()
            .join("scheduler.txt");

        assert_eq!(std::fs::read_to_string(&path).unwrap().lines().count(), 0);

        scheduler.jobs = HashMap::from([(
            "test".to_string(),
            Job {
                kind: Kind::Shell {
                    command: format!("echo loco >> {}", path.display()),
                },
                cron: "*/1 * * * * *".to_string(),
                tags: None,
                output: None,
            },
        )]);

        let handle = tokio::spawn(async move {
            scheduler.run().await.unwrap();
        });

        time::sleep(Duration::from_secs(5)).await;
        handle.abort();

        assert!(std::fs::read_to_string(&path).unwrap().lines().count() >= 4);
    }
}
