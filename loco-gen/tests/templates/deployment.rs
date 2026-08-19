use insta::assert_snapshot;
use loco_gen::{collect_messages, generate, AppInfo, Component, DeploymentKind};
use rrgen::RRgen;
use std::{fs, path::PathBuf};

/// The generated Dockerfile pins its own Rust base image, and nothing tied
/// that pin to the MSRV the app's dependencies actually require. It drifted:
/// the 1.0 release raised the MSRV to 1.94 for Sea-ORM 2.0 and left the
/// template on 1.92.0, so every generated Dockerfile failed at
/// `cargo build --release` with "rustc 1.92.0 is not supported". The nightly
/// `loco-gen-deploy` workflow reported it every night and the snapshot test
/// could not: it filters the version line out before comparing.
fn assert_dockerfile_rust_meets_msrv(dockerfile: &str) {
    let pinned = dockerfile
        .lines()
        .find_map(|line| {
            line.strip_prefix("FROM rust:")
                .and_then(|rest| rest.split('-').next())
        })
        .expect("Dockerfile has no `FROM rust:<version>` line");

    let parts = |v: &str| -> Vec<u64> { v.split('.').map(|p| p.parse().unwrap_or(0)).collect() };
    // `rust-version` may be a two-component "1.94"; compare on what it gives.
    let msrv = env!("CARGO_PKG_RUST_VERSION");
    let (pinned_parts, msrv_parts) = (parts(pinned), parts(msrv));

    assert!(
        pinned_parts >= msrv_parts[..msrv_parts.len().min(pinned_parts.len())].to_vec(),
        "generated Dockerfile pins rust {pinned}, but loco-gen declares rust-version {msrv} — \
         the image cannot build the app it generates. Bump \
         `loco-gen/src/templates/deployment/docker/docker.t`."
    );
}

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
            working_dir: tree_fs.root.clone(),
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

    assert_dockerfile_rust_meets_msrv(
        &fs::read_to_string(tree_fs.root.join("Dockerfile")).expect("Dockerfile missing"),
    );

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
            working_dir: tree_fs.root.clone(),
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

#[rstest::rstest]
fn can_generate_lambda(#[values(true, false)] db: bool) {
    let mut settings = insta::Settings::clone_current();
    settings.set_prepend_module_to_snapshot(false);
    settings.set_snapshot_suffix("deployment");
    let _guard = settings.bind_to_scope();

    let component = Component::Deployment {
        kind: DeploymentKind::Lambda {
            db,
            include_paths: vec![],
        },
    };

    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add(
            "Cargo.toml",
            "[dependencies]\nloco-rs = { version = \"1\" }\n",
        )
        .create()
        .unwrap();
    let rrgen = RRgen::with_working_dir(&tree_fs.root);

    let gen_result = generate(
        &rrgen,
        component,
        &AppInfo {
            app_name: "tester".to_string(),
            working_dir: tree_fs.root.clone(),
        },
    )
    .expect("Generation failed");

    assert_eq!(
        collect_messages(&gen_result),
        "* Lambda entrypoint + cargo-lambda config generated. Deploy in two commands: `cargo lambda build --release --arm64 --output-format zip` then `cargo lambda deploy --enable-function-url` (prints a live HTTPS URL).\n"
    );

    let cargo = fs::read_to_string(tree_fs.root.join("Cargo.toml")).expect("Cargo.toml missing");
    assert!(
        cargo.contains(r#"lambda_http = { version = "1.3" }"#),
        "expected lambda_http dependency injected, got:\n{cargo}"
    );
    // Declarative cargo-lambda config so build/deploy need no extra flags.
    assert!(
        cargo.contains("[package.metadata.lambda.build]")
            && cargo.contains(r#"include = ["config"]"#),
        "expected lambda build metadata with config include, got:\n{cargo}"
    );
    assert!(
        cargo.contains("[package.metadata.lambda.deploy]")
            && cargo.contains(r#"env = { LOCO_ENV = "production" }"#),
        "expected lambda deploy metadata, got:\n{cargo}"
    );

    assert_snapshot!(
        format!("generate[lambda_[{db}]]"),
        fs::read_to_string(tree_fs.root.join("src").join("bin").join("lambda.rs"))
            .expect("lambda entrypoint missing")
    );
}
