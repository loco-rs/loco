use std::fs;

use insta::assert_snapshot;
use loco_gen::{collect_messages, generate, AppInfo, Component};
use rrgen::RRgen;

#[test]
fn can_generate() {
    let mut settings = insta::Settings::clone_current();
    settings.set_prepend_module_to_snapshot(false);
    settings.set_snapshot_suffix("mailer");
    let _guard = settings.bind_to_scope();

    let component = Component::Mailer {
        name: "reset_password".to_string(),
    };

    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add_empty("src/mailers/mod.rs")
        .create()
        .unwrap();

    let rrgen = RRgen::with_working_dir(&tree_fs.root);

    let gen_result = generate(
        &rrgen,
        component,
        &AppInfo {
            app_name: "tester".to_string(),
        },
    )
    .expect("Generation failed");

    assert_eq!(
        collect_messages(&gen_result),
        r"* A mailer `ResetPassword` was added successfully.
"
    );

    let mailer_path = tree_fs.root.join("src").join("mailers");

    for (name, path) in [
        (
            "generate[mailer_mod_rs]",
            mailer_path.join("reset_password.rs"),
        ),
        ("inject[mailer_mod_rs]", mailer_path.join("mod.rs")),
        (
            "generate[subject_t_file]",
            mailer_path
                .join("reset_password")
                .join("welcome")
                .join("subject.t"),
        ),
        (
            "generate[text_t_file]",
            mailer_path
                .join("reset_password")
                .join("welcome")
                .join("text.t"),
        ),
        (
            "generate[html_t_file]",
            mailer_path
                .join("reset_password")
                .join("welcome")
                .join("html.t"),
        ),
    ] {
        assert_snapshot!(
            name,
            fs::read_to_string(path).unwrap_or_else(|_| panic!("{name} missing"))
        );
    }
}
