use crate::prelude::*;

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
        println!("Foo task executed!!!");
        Ok(())
    }
}

pub struct ParseArgs;
#[async_trait]
impl Task for ParseArgs {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "parse_args".to_string(),
            detail: "Validate the paring args".to_string(),
        }
    }
    async fn run(&self, _app_context: &AppContext, vars: &task::Vars) -> Result<()> {
        let refresh = vars.cli_arg("test").is_ok_and(|test| test == "true");

        let app = vars
            .cli_arg("app")
            .map(std::string::ToString::to_string)
            .unwrap_or_default();

        if refresh && app == "loco" {
            Ok(())
        } else {
            Err(Error::string("invalid args"))
        }
    }
}
