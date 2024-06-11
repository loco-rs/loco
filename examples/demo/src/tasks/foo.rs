// <snip id="task-code-example" />
use loco_rs::prelude::*;

pub struct Foo;
#[async_trait]
impl Task for Foo {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "foo".to_string(),
            detail: "run foo task".to_string(),
        }
    }
    async fn run(&self, _app_context: &AppContext, _vars: &task::Vars) -> Result<()> {
        Ok(())
    }
}
// </snip>
