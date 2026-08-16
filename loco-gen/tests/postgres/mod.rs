//! A Postgres to run the migration flow against.
//!
//! `loco-gen` cannot reuse `loco_rs::tests_cfg::postgres` — `loco-rs` depends
//! on `loco-gen`, not the other way round — so this is the same idea, smaller:
//! no sqlx, because the thing that proves the database is usable is the
//! `loco db reset` that runs seconds later.
//!
//! An explicit `DATABASE_URL` wins. That keeps a local Postgres or a CI
//! service container as the fast path, and means this file starts a container
//! only when there is nothing else to talk to.

use std::{
    net::TcpStream,
    time::{Duration, Instant},
};

use testcontainers::{
    core::{ContainerPort, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, GenericImage, ImageExt,
};

pub struct Postgres {
    pub url: String,
    /// Both `None` on the `DATABASE_URL` path — there is no container to own.
    /// Torn down by the hand-written `Drop` below rather than by field order.
    container: Option<ContainerAsync<GenericImage>>,
    runtime: Option<tokio::runtime::Runtime>,
}

impl Drop for Postgres {
    fn drop(&mut self) {
        let (Some(container), Some(runtime)) = (self.container.take(), self.runtime.take()) else {
            return;
        };

        // `ContainerAsync`'s own `Drop` calls `tokio::runtime::Handle::current()`
        // to schedule the container's removal. Keeping the runtime alive is not
        // the same as being inside it: without `enter()` that call panics with
        // "there is no reactor running", the test fails *after* everything it
        // asserts has already passed, and the container is left running.
        {
            let _guard = runtime.enter();
            drop(container);
        }
        // Outside the guard: dropping a runtime from within its own context
        // panics in turn.
        drop(runtime);
    }
}

/// # Panics
///
/// If no `DATABASE_URL` is set and a container cannot be started or does not
/// begin accepting connections. Failing is the point: this test used to skip
/// silently and report `ok`.
#[must_use]
pub fn url_or_container() -> Postgres {
    if let Ok(url) = std::env::var("DATABASE_URL") {
        println!("test_migrations_flow[postgres]: using DATABASE_URL");
        return Postgres {
            url,
            container: None,
            runtime: None,
        };
    }

    println!("test_migrations_flow[postgres]: no DATABASE_URL, starting a container");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a tokio runtime for the container");

    let (container, port) = runtime.block_on(async {
        let container = GenericImage::new("postgres", "16")
            .with_wait_for(WaitFor::message_on_stderr(
                "database system is ready to accept connections",
            ))
            .with_exposed_port(ContainerPort::Tcp(5432))
            .with_env_var("POSTGRES_USER", "postgres")
            .with_env_var("POSTGRES_PASSWORD", "postgres")
            .with_env_var("POSTGRES_DB", "postgres")
            .start()
            .await
            .expect(
                "a Postgres container — is a Docker daemon running? With Colima, \
                 DOCKER_HOST must point at ~/.colima/default/docker.sock",
            );

        let port = container
            .get_host_port_ipv4(5432)
            .await
            .expect("the mapped Postgres port");

        (container, port)
    });

    wait_for_port(port);

    Postgres {
        url: format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres"),
        container: Some(container),
        runtime: Some(runtime),
    }
}

/// The readiness message can appear while Postgres is still finishing its
/// first-boot shutdown/restart cycle, so the log line alone is not enough.
fn wait_for_port(port: u16) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("Postgres container never accepted a connection on port {port}");
}
