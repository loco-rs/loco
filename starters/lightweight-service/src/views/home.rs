use serde::{Deserialize, Serialize};

impl HomeResponse {
    #[must_use]
    pub fn new(app_name: &str) -> Self {
        Self {
            app_name: app_name.to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(clippy::module_name_repetitions)]
pub struct HomeResponse {
    pub app_name: String,
}
