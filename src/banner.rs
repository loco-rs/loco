use colored::Colorize;

use crate::boot::BootResult;

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

pub fn print_banner(boot_result: &BootResult) {
    let ctx = &boot_result.app_context;
    println!("{BANNER}");
    let config = &ctx.config;

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

    println!("environment: {}", ctx.environment.to_string().green());
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
    if config.logger.enable {
        println!("     logger: {}", config.logger.level.to_string().green());
    } else {
        println!("     logger: {}", "disabled".bright_red());
    }
    let mut modes = Vec::new();
    let mut servingline = Vec::new();
    if boot_result.router.is_some() {
        modes.push("server".green());
        servingline.push(format!(
            "listening on port {}",
            config.server.port.to_string().green()
        ));
    }
    if boot_result.processor.is_some() {
        modes.push("worker".green());
        servingline.push(format!("worker is {}", "online".green()));
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

    print!("\n");
    println!("{}", servingline.join("\n"));
}
