use insta::assert_snapshot;
use loco_gen::{
    collect_messages, generate, AppInfo, Component, DeploymentKind,
};
use rrgen::RRgen;
use std::{fs, path::PathBuf};

#[rstest::rstest]
fn can_generate_docker(
    #[values(vec![], vec![std::path::PathBuf::from("404.html"), PathBuf::from("asset")])]
    copy_paths: Vec<PathBuf>,
    #[values(true, false)] is_client_side_rendering: bool,
) {
    let mut settings = insta::Settings::clone_current();
    settings.set_prepend_module_to_snapshot(false);
    settings.set_snapshot_suffix("deployment");
    let _guard = settings.bind_to_scope();

    let component = Component::Deployment {
        kind: DeploymentKind::Docker {
            copy_paths: copy_paths.clone(),
            is_client_side_rendering,
        },
    };

    let tree_fs = tree_fs::TreeBuilder::default().drop(true).create().unwrap();
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
        r"* Dockerfile generated successfully.
* Dockerignore generated successfully.
"
    );
    insta::with_settings!({
        filters => vec![
            (r"FROM rust:\d+\.\d+\.\d+-slim", "FROM rust:[version]-slim"),
        ]
    }, {
        assert_snapshot!(
            format!(
                "generate[docker_file_[{}]_[{}]]",
                copy_paths.len(),
                is_client_side_rendering
            ),
            fs::read_to_string(tree_fs.root.join("Dockerfile")).expect("Dockerfile missing")
        );
    });

    assert_eq!(
        fs::read_to_string(tree_fs.root.join(".dockerignore")).expect(".dockerignore missing"),
        r"target
Dockerfile
.dockerignore
.git
.gitignore
"
    );
}

#[test]
fn can_generate_nginx() {
    let mut settings = insta::Settings::clone_current();
    settings.set_prepend_module_to_snapshot(false);
    settings.set_snapshot_suffix("deployment");
    let _guard = settings.bind_to_scope();

    let component = Component::Deployment {
        kind: DeploymentKind::Nginx {
            host: "localhost".to_string(),
            port: 8080,
        },
    };

    let tree_fs = tree_fs::TreeBuilder::default().drop(true).create().unwrap();
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
        r"* Nginx generated successfully.
"
    );
    assert_snapshot!(
        "generate[nginx]",
        fs::read_to_string(tree_fs.root.join("nginx").join("default.conf"))
            .expect("nginx config missing")
    );
}

