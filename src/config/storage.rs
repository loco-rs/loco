use serde::{Deserialize, Serialize};

/// Storage configuration for the application.
///
/// Mirrors [`crate::config::CacheConfig`]: a `kind`-tagged enum selects the
/// driver, and the operator picks it from YAML instead of wiring a
/// [`crate::storage::drivers::StoreDriver`] by hand in
/// [`crate::app::Hooks::after_context`]. That override path still works —
/// `after_context` runs after this config produces the default store, and
/// can freely replace it.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(tag = "kind")]
#[non_exhaustive]
pub enum StorageConfig {
    #[cfg(feature = "storage_aws_s3")]
    /// AWS S3 storage
    Aws(AwsStorageConfig),
    #[cfg(feature = "storage_gcp")]
    /// Google Cloud Storage
    Gcp(GcpStorageConfig),
    #[cfg(feature = "storage_azure")]
    /// Azure Blob storage
    Azure(AzureStorageConfig),
    /// Local filesystem storage
    Local(LocalStorageConfig),
    /// Null storage: every operation errors. The default when no `storage:`
    /// block is present, matching today's hardcoded behavior.
    #[default]
    Null,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LocalStorageConfig {
    /// Root directory for stored files. Defaults to the process working
    /// directory when omitted.
    pub path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AwsStorageConfig {
    pub bucket: String,
    pub region: String,
    pub endpoint: Option<String>,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub session_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GcpStorageConfig {
    pub bucket: String,
    pub credential_path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AzureStorageConfig {
    pub container: String,
    pub account_name: String,
    pub access_key: String,
    pub endpoint: String,
}
