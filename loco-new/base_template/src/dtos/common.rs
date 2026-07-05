use ts_rs::TS;

#[derive(serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct Page<T: TS> {
    pub items: Vec<T>,
    #[ts(type = "number")]
    pub total: i64,
    #[ts(type = "number")]
    pub page: i64,
    #[ts(type = "number")]
    pub per_page: i64,
}

#[derive(serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct ApiError {
    pub code: String,
    pub message: String,
    #[ts(type = "unknown")]
    pub details: Option<serde_json::Value>,
}
