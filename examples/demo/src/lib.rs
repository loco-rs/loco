pub mod app;
pub mod common;
pub mod controllers {
    automod::dir!(pub "src/controllers");
}
pub mod mailers {
    automod::dir!(pub "src/mailers");
}
pub mod models;
pub mod tasks {
    automod::dir!(pub "src/tasks");
}
pub mod views;
pub mod workers {
    automod::dir!(pub "src/workers");
}
