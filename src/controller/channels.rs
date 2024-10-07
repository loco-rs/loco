//! This module defines the `AppChannels` struct and its related implementations
//! using the `socketioxide` crate for handling socket IO.
use socketioxide::{layer::SocketIoLayer, SocketIo, SocketIoBuilder};

/// Struct representing channels for the application, including a socket IO
/// layer and a socket IO instance for registration.
#[derive(Clone, Debug)]
pub struct AppChannels {
    /// Socket IO layer for managing communication channels.
    pub layer: SocketIoLayer,
    /// Socket IO instance for registration and communication.
    pub register: SocketIo,
}

/// Implementation of the Into trait for converting a `SocketIoBuilder` into
/// `AppChannels`.
impl From<SocketIoBuilder> for AppChannels {
    fn from(val: SocketIoBuilder) -> Self {
        let (layer, io) = val.build_layer();
        Self {
            layer,
            register: io,
        }
    }
}

impl AppChannels {
    /// Creates a new `SocketIoBuilder` using builder
    #[must_use]
    pub fn builder() -> SocketIoBuilder {
        SocketIo::builder()
    }
}

/// Implementation of the Default trait for `AppChannels`.
impl Default for AppChannels {
    /// Creates a default instance of `AppChannels` with default values for the
    /// layer and socket IO.
    fn default() -> Self {
        let (layer, io) = SocketIo::new_layer();

        Self {
            layer,
            register: io,
        }
    }
}
