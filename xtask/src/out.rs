use tabled::settings::Style;

use crate::ci::RunResults;

pub fn ci_results(result: &Vec<RunResults>) -> String {
    let mut builder = tabled::builder::Builder::default();

    builder.push_record(vec!["path", "fmt", "clippy", "test"]);

    for ci in result {
        builder.push_record(vec![
            format!("{}", ci.path.display()),
            ci.fmt.to_string(),
            ci.clippy.to_string(),
            ci.test.to_string(),
        ]);
    }

    builder.build().with(Style::modern()).to_string()
}
