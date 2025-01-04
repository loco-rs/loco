use crate::env_vars;





pub const DEFAULT_ENVIRONMENT: &str = "development";
pub const LOCO_ENV: &str = "LOCO_ENV";
pub const RAILS_ENV: &str = "RAILS_ENV";
pub const NODE_ENV: &str = "NODE_ENV";

#[must_use]
pub fn resolve_from_env() -> String {
    env_vars::get(env_vars::LOCO_ENV)
        .or_else(|_| env_vars::get(env_vars::RAILS_ENV))
        .or_else(|_| env_vars::get(env_vars::NODE_ENV))
        .unwrap_or_else(|_| DEFAULT_ENVIRONMENT.to_string())
}

pub fn resolve_config_folder() {
    
}