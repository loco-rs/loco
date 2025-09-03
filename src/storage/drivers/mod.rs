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

/// TODO: Add more methods to `GetResponse` to read the content in different
/// ways
///
/// For example, we can read a specific range of bytes from the stream.
pub struct GetResponse {
    stream: Reader,
}

impl GetResponse {
    pub(crate) fn new(stream: Reader) -> Self {
        Self { stream }
    }

    /// Read all content from the stream and return as `Bytes`.
    ///
    /// # Errors
    ///
    /// Returns a `StorageError` with the reason for the failure.
    pub async fn bytes(&self) -> StorageResult<Bytes> {
        Ok(self.stream.read(..).await?.to_bytes())
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
        BytesStream::from_reader(self.stream).await
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
