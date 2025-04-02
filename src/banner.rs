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
        let mut database = Vec::new();
        if config.database.enable_logging {
            database.push("logging".green());
        }
        if config.database.auto_migrate {
            database.push("automigrate".yellow());
        }
        if config.database.dangerously_recreate {
            database.push("recreate".bright_red());
        }
        if config.database.dangerously_truncate {
            database.push("truncate".bright_red());
        }

        if !database.is_empty() {
            println!(
                "   database: {}",
                database
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
    if config.logger.enable {
        println!("     logger: {}", config.logger.level.to_string().green());
    } else {
        println!("     logger: {}", "disabled".bright_red());
    }
    if cfg!(debug_assertions) {
        println!("compilation: {}", "debug".bright_red());
    } else {
        println!("compilation: {}", "release".green());
    }

    let mut modes = Vec::new();
    let mut servingline = Vec::new();
    if boot_result.router.is_some() {
        modes.push("server".green());
        servingline.push(format!(
            "listening on http://{}:{}",
            server_config.binding.to_string().green(),
            server_config.port.to_string().green()
        ));
    }
    if boot_result.run_worker {
        modes.push("worker".green());
        servingline.push(format!("worker is {}", "online".green()));
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
