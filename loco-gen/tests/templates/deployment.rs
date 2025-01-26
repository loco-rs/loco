use std::fs;

use insta::assert_snapshot;
use loco_gen::{collect_messages, generate, AppInfo, Component, DeploymentKind};
use rrgen::RRgen;

#[rstest::rstest]
fn can_generate_docker(
    #[values(None, Some("404_html".to_string()))] fallback_file: Option<String>,
    #[values(None, Some("assets".to_string()))] asset_folder: Option<String>,
) {
    let mut settings = insta::Settings::clone_current();
    settings.set_prepend_module_to_snapshot(false);
    settings.set_snapshot_suffix("deployment");
    let _guard = settings.bind_to_scope();

    let component = Component::Deployment {
        kind: DeploymentKind::Docker,
        fallback_file: fallback_file.clone(),
        asset_folder: asset_folder.clone(),
        host: "localhost".to_string(),
        port: 8080,
        sqlite: false,
        postgres: false,
        background_queue: false,
    };

    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add(
            "config/development.yaml",
            r#"# Web server configuration
server:
# Port on which the server will listen. the server binding is 0.0.0.0:{PORT}
port: 5150
# The UI hostname or IP address that mailers will point to.
host: http://localhost
"#,
        )
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

    // assert_snapshot!("generate_docker_result", collect_messages(&gen_result));

    assert_eq!(
        collect_messages(&gen_result),
        r"* Dockerfile generated successfully.
* Dockerignore generated successfully.
"
    );
    assert_snapshot!(
        format!(
            "generate[docker_file_[{}]_[{}]]",
            fallback_file.as_ref().map_or("None", |f| f.as_str()),
            asset_folder.as_ref().map_or("None", |a| a.as_str())
        ),
        fs::read_to_string(tree_fs.root.join("dockerfile")).expect("dockerfile missing")
    );

    assert_eq!(
        fs::read_to_string(tree_fs.root.join(".dockerignore")).expect(".dockerignore missing"),
        r"target
dockerfile
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
        kind: DeploymentKind::Nginx,
        fallback_file: None,
        asset_folder: None,
        host: "localhost".to_string(),
        port: 8080,
        sqlite: false,
        postgres: false,
        background_queue: false,
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

#[test]
fn can_generate_shuttle() {
    let mut settings = insta::Settings::clone_current();
    settings.set_prepend_module_to_snapshot(false);
    settings.set_snapshot_suffix("deployment");
    let _guard = settings.bind_to_scope();

    let component = Component::Deployment {
        kind: DeploymentKind::Shuttle,
        fallback_file: None,
        asset_folder: None,
        host: "localhost".to_string(),
        port: 8080,
        sqlite: false,
        postgres: false,
        background_queue: false,
    };

    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add(
            ".cargo/config.toml",
            r#"[alias]
loco = "run --"
loco-tool = "run --"

playground = "run --example playground"
"#,
        )
        .add(
            "Cargo.toml",
            r"
[dependencies]

[dev-dependencies]

",
        )
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
        r"* Shuttle.toml file created successfully
* Shuttle deployment ready do use
"
    );
    assert_snapshot!(
        "generate[shuttle.rs]",
        fs::read_to_string(tree_fs.root.join("src").join("bin").join("shuttle.rs"))
            .expect("shuttle rs missing")
    );
    assert_snapshot!(
        "inject[.config_toml]",
        fs::read_to_string(tree_fs.root.join(".cargo").join("config.toml"))
            .expect(".cargo/config.toml not exists")
    );
    assert_snapshot!(
        "inject[cargo_toml]",
        fs::read_to_string(tree_fs.root.join("Cargo.toml")).expect("cargo.toml not exists")
    );
}
#[test]
fn can_generate_kamal_sqlite_without_background_queue() {
    let mut settings = insta::Settings::clone_current();
    settings.set_prepend_module_to_snapshot(false);
    settings.set_snapshot_suffix("deployment");
    let _guard = settings.bind_to_scope();

    let component = Component::Deployment {
        kind: DeploymentKind::Kamal,
        fallback_file: Some("404.html".to_string()),
        asset_folder: Some("assets".to_string()),
        host: "localhost".to_string(),
        port: 8080,
        sqlite: true,
        postgres: false,
        background_queue: false,
    };

    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add(
            "config/development.yaml",
            r#"# Web server configuration
server:
  # Port on which the server will listen. the server binding is 0.0.0.0:{PORT}
  port: 5150
  # The UI hostname or IP address that mailers will point to.
  host: http://localhost
"#,
        )
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
        r"* Dockerfile generated successfully.
* Dockerignore generated successfully.
* Deploy file generated successfully.
* Secrets file generated successfully.
"
    );

    assert_snapshot!(
        "inject[development.yaml]",
        fs::read_to_string(tree_fs.root.join("config").join("development.yaml"))
            .expect("development.yaml missing")
    );

    // Assert the generated Dockerfile content
    assert_snapshot!(
        "generate[kamal_dockerfile_sqlite]",
        fs::read_to_string(tree_fs.root.join("dockerfile")).expect("Dockerfile missing")
    );

    // Assert the generated .dockerignore content
    assert_snapshot!(
        "generate[kamal_dockerignore_sqlite]",
        fs::read_to_string(tree_fs.root.join(".dockerignore")).expect(".dockerignore missing")
    );

    // Assert the generated deploy.yml content
    assert_snapshot!(
        "generate[kamal_deploy_yml_sqlite]",
        fs::read_to_string(tree_fs.root.join("config").join("deploy.yml"))
            .expect("deploy.yml missing")
    );

    // Assert the generated secrets file content
    assert_snapshot!(
        "generate[kamal_secrets_sqlite]",
        fs::read_to_string(tree_fs.root.join(".kamal").join("secrets"))
            .expect("secrets file missing")
    );
}

#[test]
fn can_generate_kamal_sqlite_with_background_queue() {
    let mut settings = insta::Settings::clone_current();
    settings.set_prepend_module_to_snapshot(false);
    settings.set_snapshot_suffix("deployment");
    let _guard = settings.bind_to_scope();

    let component = Component::Deployment {
        kind: DeploymentKind::Kamal,
        fallback_file: Some("404.html".to_string()),
        asset_folder: Some("assets".to_string()),
        host: "localhost".to_string(),
        port: 8080,
        sqlite: true,
        postgres: false,
        background_queue: true,
    };

    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add(
            "config/development.yaml",
            r#"# Web server configuration
server:
  # Port on which the server will listen. the server binding is 0.0.0.0:{PORT}
  port: 5150
  # The UI hostname or IP address that mailers will point to.
  host: http://localhost
"#,
        )
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
        r"* Dockerfile generated successfully.
* Dockerignore generated successfully.
* Deploy file generated successfully.
* Secrets file generated successfully.
"
    );
    assert_snapshot!(
        "inject[development.yaml]",
        fs::read_to_string(tree_fs.root.join("config").join("development.yaml"))
            .expect("development.yaml missing")
    );
    // Assert the generated dockerfile content
    assert_snapshot!(
        "generate[kamal_dockerfile_sqlite_with_background_queue]",
        fs::read_to_string(tree_fs.root.join("dockerfile")).expect("Dockerfile missing")
    );

    // Assert the generated .dockerignore content
    assert_snapshot!(
        "generate[kamal_dockerignore_sqlite_with_background_queue]",
        fs::read_to_string(tree_fs.root.join(".dockerignore")).expect(".dockerignore missing")
    );

    // Assert the generated deploy.yml content
    assert_snapshot!(
        "generate[kamal_deploy_yml_sqlite_with_background_queue]",
        fs::read_to_string(tree_fs.root.join("config").join("deploy.yml"))
            .expect("deploy.yml missing")
    );

    // Assert the generated secrets file content
    assert_snapshot!(
        "generate[kamal_secrets_sqlite_with_background_queue]",
        fs::read_to_string(tree_fs.root.join(".kamal").join("secrets"))
            .expect("secrets file missing")
    );
}

#[test]
fn can_generate_kamal_postgres_without_background_queue() {
    let mut settings = insta::Settings::clone_current();
    settings.set_prepend_module_to_snapshot(false);
    settings.set_snapshot_suffix("deployment");
    let _guard = settings.bind_to_scope();

    let component = Component::Deployment {
        kind: DeploymentKind::Kamal,
        fallback_file: Some("404.html".to_string()),
        asset_folder: Some("assets".to_string()),
        host: "localhost".to_string(),
        port: 8080,
        sqlite: false,
        postgres: true,
        background_queue: false,
    };

    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add(
            "config/development.yaml",
            r#"# Web server configuration
server:
  # Port on which the server will listen. the server binding is 0.0.0.0:{PORT}
  port: 5150
  # The UI hostname or IP address that mailers will point to.
  host: http://localhost
"#,
        )
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
        r"* Dockerfile generated successfully.
* Dockerignore generated successfully.
* Deploy file generated successfully.
* Secrets file generated successfully.
"
    );
    assert_snapshot!(
        "inject[development.yaml]",
        fs::read_to_string(tree_fs.root.join("config").join("development.yaml"))
            .expect("development.yaml missing")
    );
    // Assert the generated dockerfile content
    assert_snapshot!(
        "generate[kamal_dockerfile_postgres_without_background_queue]",
        fs::read_to_string(tree_fs.root.join("dockerfile")).expect("Dockerfile missing")
    );

    // Assert the generated .dockerignore content
    assert_snapshot!(
        "generate[kamal_dockerignore_postgres_without_background_queue]",
        fs::read_to_string(tree_fs.root.join(".dockerignore")).expect(".dockerignore missing")
    );

    // Assert the generated deploy.yml content
    assert_snapshot!(
        "generate[kamal_deploy_yml_postgres_without_background_queue]",
        fs::read_to_string(tree_fs.root.join("config").join("deploy.yml"))
            .expect("deploy.yml missing")
    );

    // Assert the generated secrets file content
    assert_snapshot!(
        "generate[kamal_secrets_postgres_without_background_queue]",
        fs::read_to_string(tree_fs.root.join(".kamal").join("secrets"))
            .expect("secrets file missing")
    );
}

#[test]
fn can_generate_kamal_postgres_with_background_queue() {
    let mut settings = insta::Settings::clone_current();
    settings.set_prepend_module_to_snapshot(false);
    settings.set_snapshot_suffix("deployment");
    let _guard = settings.bind_to_scope();

    let component = Component::Deployment {
        kind: DeploymentKind::Kamal,
        fallback_file: Some("404.html".to_string()),
        asset_folder: Some("assets".to_string()),
        host: "localhost".to_string(),
        port: 8080,
        sqlite: false,
        postgres: true,
        background_queue: true,
    };

    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add(
            "config/development.yaml",
            r#"# Web server configuration
server:
  # Port on which the server will listen. the server binding is 0.0.0.0:{PORT}
  port: 5150
  # The UI hostname or IP address that mailers will point to.
  host: http://localhost
"#,
        )
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
        r"* Dockerfile generated successfully.
* Dockerignore generated successfully.
* Deploy file generated successfully.
* Secrets file generated successfully.
"
    );
    assert_snapshot!(
        "inject[development.yaml]",
        fs::read_to_string(tree_fs.root.join("config").join("development.yaml"))
            .expect("development.yaml missing")
    );
    // Assert the generated dockerfile content
    assert_snapshot!(
        "generate[kamal_dockerfile_postgres_with_background_queue]",
        fs::read_to_string(tree_fs.root.join("dockerfile")).expect("Dockerfile missing")
    );

    // Assert the generated .dockerignore content
    assert_snapshot!(
        "generate[kamal_dockerignore_postgres_with_background_queue]",
        fs::read_to_string(tree_fs.root.join(".dockerignore")).expect(".dockerignore missing")
    );

    // Assert the generated deploy.yml content
    assert_snapshot!(
        "generate[kamal_deploy_yml_postgres_with_background_queue]",
        fs::read_to_string(tree_fs.root.join("config").join("deploy.yml"))
            .expect("deploy.yml missing")
    );

    // Assert the generated secrets file content
    assert_snapshot!(
        "generate[kamal_secrets_postgres_with_background_queue]",
        fs::read_to_string(tree_fs.root.join(".kamal").join("secrets"))
            .expect("secrets file missing")
    );
}
