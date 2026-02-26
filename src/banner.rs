use colored::Colorize;

use crate::boot::{BootResult, ServeParams};

pub const BANNER: &str = r"
                      ▄     ▀                     
                                 ▀  ▄             
                  ▄       ▀     ▄  ▄ ▄▀           
                                    ▄ ▀▄▄         
                        ▄     ▀    ▀  ▀▄▀█▄       
                                          ▀█▄     
▄▄▄▄▄▄▄  ▄▄▄▄▄▄▄▄▄   ▄▄▄▄▄▄▄▄▄▄▄ ▄▄▄▄▄▄▄▄▄ ▀▀█    
 ██████  █████   ███ █████   ███ █████   ███ ▀█   
 ██████  █████   ███ █████   ▀▀▀ █████   ███ ▄█▄  
 ██████  █████   ███ █████       █████   ███ ████▄
 ██████  █████   ███ █████   ▄▄▄ █████   ███ █████
 ██████  █████   ███  ████   ███ █████   ███ ████▀
   ▀▀▀██▄ ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀ ██▀  
       ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀    
                https://loco.rs
";

pub fn print_banner(boot_result: &BootResult, server_config: &ServeParams) {
    let ctx = &boot_result.app_context;
    println!("{BANNER}");
    let config = &ctx.config;

    println!("environment: {}", ctx.environment.to_string().green());

    #[cfg(feature = "with-db")]
    {
        let db_modes = [
            (config.database.enable_logging, "logging".green()),
            (config.database.auto_migrate, "automigrate".yellow()),
            (
                config.database.dangerously_recreate,
                "recreate".bright_red(),
            ),
            (
                config.database.dangerously_truncate,
                "truncate".bright_red(),
            ),
        ]
        .iter()
        .filter(|x| x.0)
        .map(|x| x.1.to_string())
        .collect::<Vec<_>>();

        if !db_modes.is_empty() {
            println!("   database: {}", db_modes.join(", "));
        }
    }

    println!(
        "     logger: {}",
        if config.logger.enable {
            config.logger.level.to_string().green()
        } else {
            "disabled".bright_red()
        }
    );

    println!(
        "compilation: {}",
        if cfg!(debug_assertions) {
            "debug".bright_red()
        } else {
            "release".green()
        }
    );

    let mut modes = Vec::new();
    let mut servingline = Vec::new();
    if boot_result.router.is_some() {
        modes.push("server".green());
        servingline.push(format!(
            "listening on http://{}:{}",
            server_config.binding.clone().green(),
            server_config.port.to_string().green()
        ));
    }
    if let Some(tags) = &boot_result.worker {
        modes.push("worker".green());
        let status = format!("worker is {}", "online".green());
        if tags.is_empty() {
            servingline.push(status);
        } else {
            servingline.push(format!("{status} with tags: {}", tags.join(",")));
        }
    }
    if boot_result.run_scheduler {
        modes.push("scheduler".green());
        servingline.push(format!("scheduler is {}", "running".green()));
    }
    if !modes.is_empty() {
        println!(
            "      modes: {}",
            modes
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    println!();
    println!("{}", servingline.join("\n"));
}
