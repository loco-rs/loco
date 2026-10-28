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

#[derive(Clone, Deserialize, Serialize)]
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

#[derive(Clone, Deserialize, Serialize)]
pub struct AzureStorageConfig {
    pub container: String,
    pub account_name: String,
    pub access_key: String,
    pub endpoint: String,
}

// Secrets never reach logs: `Config` is `Debug`-printed in places.
impl std::fmt::Debug for AwsStorageConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let redact = |v: &Option<String>| v.as_ref().map(|_| "[redacted]");
        f.debug_struct("AwsStorageConfig")
            .field("bucket", &self.bucket)
            .field("region", &self.region)
            .field("endpoint", &self.endpoint)
            .field("access_key_id", &self.access_key_id)
            .field("secret_access_key", &redact(&self.secret_access_key))
            .field("session_token", &redact(&self.session_token))
            .finish()
    }
}

impl std::fmt::Debug for AzureStorageConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AzureStorageConfig")
            .field("container", &self.container)
            .field("account_name", &self.account_name)
            .field("access_key", &"[redacted]")
            .field("endpoint", &self.endpoint)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(yaml: &str) -> StorageConfig {
        serde_yaml::from_str(yaml).expect("valid storage yaml")
    }

    #[test]
    fn null_and_local() {
        assert!(matches!(parse("kind: Null"), StorageConfig::Null));
        let StorageConfig::Local(c) = parse("kind: Local\npath: /tmp/x") else {
            panic!("expected Local")
        };
        assert_eq!(c.path.as_deref(), Some("/tmp/x"));
        let StorageConfig::Local(c) = parse("kind: Local") else {
            panic!("expected Local")
        };
        assert!(c.path.is_none());
    }

    #[cfg(feature = "storage_aws_s3")]
    #[test]
    fn aws_optional_fields_and_empty_strings() {
        let StorageConfig::Aws(c) = parse("kind: Aws\nbucket: b\nregion: r") else {
            panic!("expected Aws")
        };
        assert!(c.access_key_id.is_none() && c.endpoint.is_none());

        let StorageConfig::Aws(c) = parse(
            "kind: Aws\nbucket: b\nregion: r\nendpoint: ''\naccess_key_id: ''\nsecret_access_key: ''",
        ) else {
            panic!("expected Aws")
        };
        assert_eq!(c.endpoint.as_deref(), Some(""));
    }

    #[cfg(feature = "storage_aws_s3")]
    #[test]
    fn debug_hides_secrets() {
        let StorageConfig::Aws(c) = parse(
            "kind: Aws\nbucket: b\nregion: r\naccess_key_id: AKID\nsecret_access_key: \
             hunter2\nsession_token: tok123",
        ) else {
            panic!("expected Aws")
        };
        let shown = format!("{c:?}");
        assert!(
            !shown.contains("hunter2") && !shown.contains("tok123"),
            "{shown}"
        );
        assert!(shown.contains("AKID"));
    }

    #[cfg(feature = "storage_gcp")]
    #[test]
    fn gcp() {
        let StorageConfig::Gcp(c) = parse("kind: Gcp\nbucket: b\ncredential_path: /k.json") else {
            panic!("expected Gcp")
        };
        assert_eq!(c.credential_path, "/k.json");
    }

    #[cfg(feature = "storage_azure")]
    #[test]
    fn azure_and_debug_hides_key() {
        let StorageConfig::Azure(c) = parse(
            "kind: Azure\ncontainer: c\naccount_name: a\naccess_key: s3cr3t\nendpoint: http://e",
        ) else {
            panic!("expected Azure")
        };
        assert_eq!(c.container, "c");
        assert!(!format!("{c:?}").contains("s3cr3t"));
    }
}
