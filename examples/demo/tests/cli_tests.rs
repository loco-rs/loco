#[test]
fn cli_tests() {
    let t = trycmd::TestCases::new();
    t.case("tests/cmd/*.trycmd")
        .extend_vars([
            (
                "[NODEPORT]",
                std::env::var("NODE_PORT").unwrap_or_else(|_| "5150".to_string()),
            ),
            (
                "[DATABASEURL]",
                std::env::var("DATABASE_URL")
                    .unwrap_or_else(|_| "postgres://loco:loco@localhost:5432/loco_app".to_string()),
            ),
            (
                "[REDISURL]",
                std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1".to_string()),
            ),
            (
                "[MAILERHOST]",
                std::env::var("MAILER_HOST").unwrap_or_else(|_| "localhost".to_string()),
            ),
        ])
        .unwrap();
}
