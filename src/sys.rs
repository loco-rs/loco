use tokio::{select, signal, task::JoinHandle};

pub async fn wait_with_force_quit(handle: JoinHandle<()>) {
    println!("press ctrl-c again to force quit");
    select! {
        _ = handle => {}
        () = wait_for_ctrlc_or_term() => {}
    }
}

/// Waits for a shutdown signal, either via Ctrl+C or termination signal.
///
/// # Panics
///
/// This function will panic if it fails to install the signal handlers for
/// Ctrl+C or the terminate signal on Unix-based systems.
pub async fn wait_for_ctrlc_or_term() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}
