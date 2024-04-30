#[cfg(feature = "with-db")]
#[deprecated(
    since = "0.3.2",
    note = "reshape pagination functionality by moving under models. read more https://loco.rs/docs/the-app/pagination"
)]
pub mod pagination;
