use std::env;
#[test]
fn cli_tests() {
    trycmd::TestCases::new().case("tests/cmd/*.trycmd");
}

#[test]
fn cli_starters_tests() {
    if env::var("LOCO_CI_MODE").is_ok() {
        trycmd::TestCases::new().case("tests/cmd/starters/*.trycmd");
    }
}
