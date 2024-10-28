use colored::Colorize;

use crate::generate::{AssetsOption, BackgroundOption, DBOption};

pub fn for_options(
    dbopt: &DBOption,
    bgopt: &BackgroundOption,
    assetopt: &AssetsOption,
) -> Vec<String> {
    let mut res = Vec::new();
    match dbopt {
        DBOption::Postgres => {
            res.push(format!(
                "{}: You've selected `{}` as your DB provider (you should have a postgres \
                 instance to connect to)",
                "database".underline(),
                "postgres".yellow()
            ));
        }
        DBOption::Sqlite => {}
    }
    match bgopt {
        BackgroundOption::Queue => res.push(format!(
            "{}: You've selected `{}` for your background worker configuration (you should have a \
             Redis/valkey instance to connect to)",
            "workers".underline(),
            "queue".yellow()
        )),
        BackgroundOption::Async => {}
        BackgroundOption::Blocking => res.push(format!(
            "{}: You've selected `{}` for your background worker configuration. Your workers \
             configuration will BLOCK REQUESTS until a task is done.",
            "workers".underline(),
            "blocking".yellow()
        )),
    }
    match assetopt {
        AssetsOption::Clientside => res.push(format!(
            "{}: You've selected `{}` for your asset serving configuration.\n\nNext step, build \
             your frontend:\n  $ cd {}\n  $ npm install && npm build\n",
            "assets".underline(),
            "clientside".yellow(),
            "frontend/".yellow()
        )),
        AssetsOption::Serverside => {}
    }
    res
}
