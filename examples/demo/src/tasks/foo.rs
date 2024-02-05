use std::collections::BTreeMap;

use loco_rs::prelude::*;

pub struct Foo;
#[async_trait]
impl Task for Foo {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "foo".to_string(),
            detail: "test misaligned cli prints".to_string(),
        }
    }
    async fn run(&self, _app_context: &AppContext, _vars: &BTreeMap<String, String>) -> Result<()> {
        Ok(())
    }
}
