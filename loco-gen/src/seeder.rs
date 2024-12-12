use super::{collect_messages, Result};
use rrgen::RRgen;
use serde_json::json;

const DEFAULT_SEEDER_T: &str = include_str!("templates/seeder/default.t");

pub fn generate(rrgen: &RRgen, name: &str) -> Result<String> {
    let vars = json!({"name": name});
    let res = rrgen.generate(DEFAULT_SEEDER_T, &vars)?;

    Ok(collect_messages(vec![res]))
}
