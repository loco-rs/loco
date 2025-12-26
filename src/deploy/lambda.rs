//! AWS Lambda deployment utilities using Cargo Lambda
//!
//! This module handles the deployment of Loco applications to AWS Lambda.
//! It creates temporary files, builds the Lambda function, deploys to AWS,
//! and cleans up automatically.

use std::path::PathBuf;

use colored::Colorize;
use duct::cmd;

use crate::{config::Lambda as LambdaConfig, Error, Result};

/// Lambda handler template for wrapping Loco applications
const HANDLER_TEMPLATE: &str = include_str!("lambda_handler.rs.tpl");

/// Manages temporary Lambda deployment files with automatic cleanup
pub struct DeployContext {
    lambda_bin_path: PathBuf,
    cargo_backup: Option<String>,
    cargo_path: PathBuf,
}

impl DeployContext {
    /// Creates a new deployment context
    #[must_use]
    pub fn new() -> Self {
        Self {
            lambda_bin_path: PathBuf::from("src/bin/_loco_lambda.rs"),
            cargo_backup: None,
            cargo_path: PathBuf::from("Cargo.toml"),
        }
    }

    /// Sets up temporary files for Lambda deployment
    ///
    /// # Errors
    ///
    /// Returns error if file operations fail
    pub fn setup(&mut self, app_name: &str, with_db: bool) -> Result<()> {
        let handler = HANDLER_TEMPLATE
            .replace("{{APP_MODULE}}", app_name)
            .replace(
                "{{MIGRATION_IMPORT}}",
                if with_db {
                    "use migration::Migrator;"
                } else {
                    ""
                },
            )
            .replace(
                "{{MIGRATOR_GENERIC}}",
                if with_db { ", Migrator" } else { "" },
            );

        if let Some(parent) = self.lambda_bin_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.lambda_bin_path, handler)?;

        let cargo_content = std::fs::read_to_string(&self.cargo_path)?;
        self.cargo_backup = Some(cargo_content.clone());

        let mut new_cargo = cargo_content;

        if !new_cargo.contains("lambda_http") {
            let dep_insert = r#"
# Lambda dependencies (auto-added by loco deploy)
lambda_http = "0.14"
lambda_runtime = "0.14"
tower = "0.4"
"#;
            if let Some(pos) = new_cargo.find("[dependencies]") {
                let insert_pos = new_cargo[pos..]
                    .find('\n')
                    .map_or(new_cargo.len(), |p| pos + p + 1);
                new_cargo.insert_str(insert_pos, dep_insert);
            }
        }

        if !new_cargo.contains("_loco_lambda") {
            let bin_section = r#"

[[bin]]
name = "_loco_lambda"
path = "src/bin/_loco_lambda.rs"
"#;
            new_cargo.push_str(bin_section);
        }

        std::fs::write(&self.cargo_path, new_cargo)?;
        Ok(())
    }

    /// Cleans up temporary files
    ///
    /// # Errors
    ///
    /// Returns error if file operations fail
    pub fn cleanup(&self) -> Result<()> {
        if self.lambda_bin_path.exists() {
            std::fs::remove_file(&self.lambda_bin_path)?;
        }

        if let Some(backup) = &self.cargo_backup {
            std::fs::write(&self.cargo_path, backup)?;
        }

        Ok(())
    }
}

impl Default for DeployContext {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for DeployContext {
    fn drop(&mut self) {
        if let Err(e) = self.cleanup() {
            eprintln!("Warning: Failed to cleanup Lambda deploy files: {e}");
        }
    }
}

/// Deploys the application to AWS Lambda
///
/// # Errors
///
/// Returns error if build or deployment fails
pub fn deploy(
    config: &LambdaConfig,
    project_name: &str,
    app_name: &str,
    with_db: bool,
    dry_run: bool,
) -> Result<()> {
    println!("{}", "🚀 Starting Lambda deployment...".green().bold());
    println!("{}", "   Creating temporary Lambda handler...".cyan());

    let mut ctx = DeployContext::new();
    ctx.setup(app_name, with_db)?;

    println!("{}", "   Building Lambda function...".cyan());

    let mut build_args = vec!["lambda", "build", "--release", "--bin", "_loco_lambda"];
    if config.architecture == "arm64" {
        build_args.push("--arm64");
    }

    cmd("cargo", &build_args)
        .run()
        .map_err(|err| Error::Message(format!("Failed to build Lambda function: {err}")))?;

    if dry_run {
        println!(
            "{}",
            "Dry run - build completed, skipping deploy.".yellow()
        );
        println!("{}", "   Cleaning up temporary files...".cyan());
        return Ok(());
    }

    println!(
        "{}",
        format!("   Deploying {project_name} to AWS Lambda...").cyan()
    );

    let mut deploy_args = vec![
        "lambda".to_string(),
        "deploy".to_string(),
        project_name.to_string(),
        format!("--region={}", config.region),
        format!("--memory={}", config.memory_size),
        format!("--timeout={}", config.timeout),
        "--binary-name=_loco_lambda".to_string(),
        "--include=config/".to_string(),
    ];

    if let Some(profile) = &config.profile_name {
        deploy_args.push(format!("--profile={profile}"));
    }

    if config.function_url {
        deploy_args.push("--enable-function-url".to_string());
    }

    if !config.environment.contains_key("LOCO_ENV") {
        deploy_args.push(format!("--env-var=LOCO_ENV={}", config.loco_env));
    }

    for (key, value) in &config.environment {
        deploy_args.push(format!("--env-var={key}={value}"));
    }

    if let Some(role_arn) = &config.role_arn {
        deploy_args.push(format!("--iam-role={role_arn}"));
    }

    let args_refs: Vec<&str> = deploy_args.iter().map(String::as_str).collect();
    cmd("cargo", &args_refs)
        .run()
        .map_err(|err| Error::Message(format!("Failed to deploy Lambda function: {err}")))?;

    println!("{}", "   Cleaning up temporary files...".cyan());
    println!("{}", "✅ Deployment completed successfully!".green().bold());

    if config.function_url {
        print_function_url(config, project_name);
    }

    Ok(())
}

/// Invokes a deployed Lambda function
///
/// # Errors
///
/// Returns error if invocation fails
pub fn invoke(config: &LambdaConfig, project_name: &str, payload: &str) -> Result<()> {
    println!(
        "{}",
        format!("Invoking Lambda function {project_name}...")
            .green()
            .bold()
    );

    let mut args = vec![
        "lambda",
        "invoke",
        project_name,
        "--region",
        &config.region,
        "--data-ascii",
        payload,
    ];

    if let Some(profile) = &config.profile_name {
        args.push("--profile");
        args.push(profile);
    }

    cmd("cargo", &args)
        .run()
        .map_err(|err| Error::Message(format!("Failed to invoke Lambda function: {err}")))?;

    Ok(())
}

/// Tails CloudWatch logs for a Lambda function
///
/// # Errors
///
/// Returns error if log fetching fails
pub fn logs(config: &LambdaConfig, project_name: &str, follow: bool) -> Result<()> {
    println!(
        "{}",
        format!("Fetching logs for {project_name}...").green().bold()
    );

    let log_group = format!("/aws/lambda/{project_name}");
    let mut args = vec!["logs", "tail", &log_group, "--region", &config.region];

    if follow {
        args.push("--follow");
    }

    if let Some(profile) = &config.profile_name {
        args.push("--profile");
        args.push(profile);
    }

    cmd("aws", &args)
        .run()
        .map_err(|err| Error::Message(format!("Failed to fetch logs: {err}")))?;

    Ok(())
}

fn print_function_url(config: &LambdaConfig, project_name: &str) {
    let mut url_args = vec![
        "lambda",
        "get-function-url-config",
        "--function-name",
        project_name,
        "--region",
        &config.region,
        "--query",
        "FunctionUrl",
        "--output",
        "text",
    ];

    let profile_str;
    if let Some(profile) = &config.profile_name {
        profile_str = profile.clone();
        url_args.push("--profile");
        url_args.push(&profile_str);
    }

    if let Ok(output) = cmd("aws", &url_args).read() {
        println!(
            "\n{} {}",
            "🌐 Function URL:".green().bold(),
            output.trim().cyan().bold()
        );
    }
}

