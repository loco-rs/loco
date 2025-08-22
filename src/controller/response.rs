use serde::Serialize;

/// Represents the health status of the application.
#[derive(Serialize)]
pub struct Health {
    pub ok: bool,
}
