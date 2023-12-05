use std::env;

#[test]
fn cli_tests() {
    if env::var("LOCO_CI_MODE").is_ok() {
        let t = trycmd::TestCases::new();
        t.case("tests/cmd/*.trycmd");
    }
}
