pub mod app;
pub mod config;
pub mod controllers;
#[cfg(feature = "with-db")]
pub mod db;
#[cfg(test)]
pub mod postgres;
#[cfg(feature = "worker")]
pub mod queue;
// The helper drives a real `redis` client, which is only linked in by the
// features below; without the gate a plain `cargo test` fails to compile.
#[cfg(all(test, any(feature = "worker_redis", feature = "cache_redis")))]
pub mod redis;
pub mod task;
