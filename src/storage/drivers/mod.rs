use std::path::Path;

use async_trait::async_trait;
use bytes::Bytes;
use opendal::Reader;

#[cfg(feature = "storage_aws_s3")]
pub mod aws;
#[cfg(feature = "storage_azure")]
pub mod azure;
#[cfg(feature = "storage_gcp")]
pub mod gcp;
pub mod local;
pub mod mem;
pub mod null;
pub mod opendal_adapter;

use super::{stream::BytesStream, StorageResult};

#[derive(Debug)]
pub struct UploadResponse {
    pub e_tag: Option<String>,
    pub version: Option<String>,
}

impl UploadResponse {
    /// Builds an `UploadResponse` from upload metadata.
    ///
    /// Custom [`StoreDriver`] implementations use this to return the result of
    /// an `upload` without relying on struct-literal construction, so the
    /// fields can later evolve behind the constructor.
    #[must_use]
    pub fn new(e_tag: Option<String>, version: Option<String>) -> Self {
        Self { e_tag, version }
    }
}

/// A single entry returned by [`StoreDriver::list`] or [`StoreDriver::stat`].
#[derive(Debug, Clone)]
pub struct ListEntry {
    /// The full path of the entry.
    pub path: String,
    /// Whether the entry is a directory (common prefix) rather than a file.
    pub is_dir: bool,
    /// The size in bytes of the entry, if known.
    pub content_length: Option<u64>,
    /// The last-modified time of the entry, if known.
    pub last_modified: Option<chrono::DateTime<chrono::Utc>>,
    /// The entity tag of the entry, if known.
    pub etag: Option<String>,
}

impl ListEntry {
    /// Builds a `ListEntry` from listing/stat metadata.
    ///
    /// Custom [`StoreDriver`] implementations use this to return listing/stat
    /// results without relying on struct-literal construction, so the fields
    /// can later evolve behind the constructor.
    #[must_use]
    pub fn new(
        path: String,
        is_dir: bool,
        content_length: Option<u64>,
        last_modified: Option<chrono::DateTime<chrono::Utc>>,
        etag: Option<String>,
    ) -> Self {
        Self {
            path,
            is_dir,
            content_length,
            last_modified,
            etag,
        }
    }
}

/// The response of a [`StoreDriver::get`] call.
///
/// Internally this is either an `OpenDAL` reader (used by the built-in,
/// opendal-backed drivers) or an already-materialized byte buffer (used by
/// custom [`StoreDriver`] implementations built via [`GetResponse::from_bytes`]).
///
/// TODO: Add more methods to `GetResponse` to read the content in different
/// ways — e.g. read a specific range of bytes from an opendal-backed response.
pub struct GetResponse {
    inner: GetResponseInner,
}

enum GetResponseInner {
    /// An opendal reader — the built-in opendal-backed drivers use this path.
    Reader(Reader),
    /// A fully-materialized buffer — custom drivers built from bytes use this.
    Bytes(Bytes),
}

impl GetResponse {
    pub(crate) fn new(stream: Reader) -> Self {
        Self {
            inner: GetResponseInner::Reader(stream),
        }
    }

    /// Builds a `GetResponse` from an already-materialized byte buffer.
    ///
    /// This is the constructor for custom [`StoreDriver`] implementations: it
    /// lets an external driver return content from `get` without depending on
    /// `opendal` types. The built-in drivers use the opendal reader path.
    #[must_use]
    pub fn from_bytes(content: Bytes) -> Self {
        Self {
            inner: GetResponseInner::Bytes(content),
        }
    }

    /// Read all content and return it as `Bytes`.
    ///
    /// # Errors
    ///
    /// Returns a `StorageError` with the reason for the failure.
    pub async fn bytes(&self) -> StorageResult<Bytes> {
        match &self.inner {
            GetResponseInner::Reader(reader) => Ok(reader.read(..).await?.to_bytes()),
            GetResponseInner::Bytes(bytes) => Ok(bytes.clone()),
        }
    }

    /// Convert the response into a streaming bytes reader.
    /// This method consumes the `GetResponse` and returns a `BytesStream`
    /// that can be used for efficient streaming without loading the entire
    /// content into memory.
    ///
    /// # Errors
    ///
    /// Returns a `StorageError` if the stream cannot be created.
    pub async fn into_stream(self) -> StorageResult<BytesStream> {
        match self.inner {
            GetResponseInner::Reader(reader) => BytesStream::from_reader(reader).await,
            GetResponseInner::Bytes(bytes) => Ok(BytesStream::from_body_stream(
                futures_util::stream::once(async move { Ok::<_, std::io::Error>(bytes) }),
            )),
        }
    }
}

#[async_trait]
pub trait StoreDriver: Sync + Send {
    /// Uploads the content represented by `Bytes` to the specified path in the
    /// object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` with the result of the upload operation.
    async fn upload(&self, path: &Path, content: &Bytes) -> StorageResult<UploadResponse>;

    /// Retrieves the content from the specified path in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` with the result of the retrieval operation.
    async fn get(&self, path: &Path) -> StorageResult<GetResponse>;

    /// Deletes the content at the specified path in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` indicating the success of the deletion
    /// operation.
    async fn delete(&self, path: &Path) -> StorageResult<()>;

    /// Renames or moves the content from one path to another in the object
    /// store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` indicating the success of the rename/move
    /// operation.
    async fn rename(&self, from: &Path, to: &Path) -> StorageResult<()>;

    /// Copies the content from one path to another in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` indicating the success of the copy operation.
    async fn copy(&self, from: &Path, to: &Path) -> StorageResult<()>;

    /// Checks if the content exists at the specified path in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` with a boolean indicating the existence of the
    /// content.
    async fn exists(&self, path: &Path) -> StorageResult<bool>;

    /// Lists entries whose paths start with the given prefix.
    ///
    /// When `recursive` is `false`, only one level below `path` is listed,
    /// with child prefixes returned as directory entries. When `recursive` is
    /// `true`, all entries under `path` are listed.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` with the listing operation's result.
    async fn list(&self, path: &Path, recursive: bool) -> StorageResult<Vec<ListEntry>>;

    /// Retrieves metadata for a single path without downloading its content.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` with the entry's metadata.
    async fn stat(&self, path: &Path) -> StorageResult<ListEntry>;

    /// Retrieves content from the specified path and returns it as a stream.
    /// This method is more memory-efficient than `get()` for large files as it
    /// doesn't load the entire content into memory.
    ///
    /// # Default Implementation
    ///
    /// The default implementation uses the regular `get()` method and converts
    /// the result to a stream. Storage drivers that support native streaming
    /// should override this method for better performance.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` with the streaming response.
    async fn get_stream(&self, path: &Path) -> StorageResult<BytesStream> {
        let response = self.get(path).await?;
        response.into_stream().await
    }

    /// Uploads content from a stream to the specified path.
    /// This method is more memory-efficient than `upload()` for large files
    /// as it doesn't require loading the entire content into memory.
    ///
    /// # Default Implementation
    ///
    /// The default implementation collects the stream into bytes and calls
    /// the regular `upload()` method. Storage drivers that support native
    /// streaming should override this method for better performance.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` with the upload response.
    async fn upload_stream(
        &self,
        path: &Path,
        stream: BytesStream,
    ) -> StorageResult<UploadResponse> {
        let bytes = stream
            .collect()
            .await
            .map_err(|e| super::StorageError::Any(Box::new(e)))?;
        self.upload(path, &bytes).await
    }
}
