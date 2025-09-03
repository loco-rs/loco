use std::path::Path;

use async_trait::async_trait;
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use opendal::{layers::RetryLayer, Operator};

use super::{GetResponse, StoreDriver, UploadResponse};
use crate::storage::{stream::BytesStream, StorageError, StorageResult};

pub struct OpendalAdapter {
    opendal_impl: Operator,
}

impl OpendalAdapter {
    /// Constructor for creating a new `Store` instance.
    #[must_use]
    pub fn new(opendal_impl: Operator) -> Self {
        let opendal_impl = opendal_impl
            // Add retry layer with default settings
            .layer(RetryLayer::default().with_jitter());
        Self { opendal_impl }
    }
}

#[async_trait]
impl StoreDriver for OpendalAdapter {
    /// Uploads the content represented by `Bytes` to the specified path in the
    /// object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` with the result of the upload operation.
    async fn upload(&self, path: &Path, content: &Bytes) -> StorageResult<UploadResponse> {
        self.opendal_impl
            .write(&path.display().to_string(), content.clone())
            .await?;
        // TODO: opendal will return the e_tag and version in the future
        Ok(UploadResponse {
            e_tag: None,
            version: None,
        })
    }

    /// Retrieves the content from the specified path in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` with the result of the retrieval operation.
    async fn get(&self, path: &Path) -> StorageResult<GetResponse> {
        let r = self
            .opendal_impl
            .reader(&path.display().to_string())
            .await?;
        Ok(GetResponse::new(r))
    }

    /// Deletes the content at the specified path in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` indicating the success of the deletion
    /// operation.
    async fn delete(&self, path: &Path) -> StorageResult<()> {
        Ok(self
            .opendal_impl
            .delete(&path.display().to_string())
            .await?)
    }

    /// Renames or moves the content from one path to another in the object
    /// store.
    ///
    /// # Behavior
    ///
    /// Fallback to copy and delete source if the storage does not support rename.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` indicating the success of the rename/move
    /// operation.
    async fn rename(&self, from: &Path, to: &Path) -> StorageResult<()> {
        if self.opendal_impl.info().full_capability().rename {
            let from = from.display().to_string();
            let to = to.display().to_string();
            Ok(self.opendal_impl.rename(&from, &to).await?)
        } else {
            self.copy(from, to).await?;
            self.delete(from).await?;
            Ok(())
        }
    }

    /// Copies the content from one path to another in the object store.
    ///
    /// # Behavior
    ///
    /// Fallback to read from source and write into dest if the storage does not support copy.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` indicating the success of the copy operation.
    async fn copy(&self, from: &Path, to: &Path) -> StorageResult<()> {
        let from = from.display().to_string();
        let to = to.display().to_string();
        if self.opendal_impl.info().full_capability().copy {
            Ok(self.opendal_impl.copy(&from, &to).await?)
        } else {
            let mut reader = self
                .opendal_impl
                .reader(&from)
                .await?
                .into_bytes_stream(..)
                .await?;
            let mut writer = self.opendal_impl.writer(&to).await?.into_bytes_sink();
            writer
                .send_all(&mut reader)
                .await
                .map_err(|err| StorageError::Any(Box::new(err)))?;
            writer
                .close()
                .await
                .map_err(|err| StorageError::Any(Box::new(err)))?;
            Ok(())
        }
    }

    /// Checks if the content exists at the specified path in the object store.
    ///
    /// # Errors
    ///
    /// Returns a `StorageResult` with a boolean indicating the existence of the
    /// content.
    ///
    /// # TODO
    ///
    /// The `exists` function should return an error for issues such as permission denied.
    /// However, these errors are not handled during the migration process and should be addressed
    /// after the test suites are refactored.
    async fn exists(&self, path: &Path) -> StorageResult<bool> {
        let path = path.display().to_string();
        Ok(self.opendal_impl.exists(&path).await.unwrap_or(false))
    }

    /// Native streaming implementation for `OpenDAL`.
    /// This directly uses `OpenDAL`'s reader for efficient streaming.
    async fn get_stream(&self, path: &Path) -> StorageResult<BytesStream> {
        let reader = self
            .opendal_impl
            .reader(&path.display().to_string())
            .await?;
        BytesStream::from_reader(reader).await
    }

    /// Native streaming upload for `OpenDAL`.
    /// This uses `OpenDAL`'s writer to stream data directly without buffering.
    async fn upload_stream(
        &self,
        path: &Path,
        stream: BytesStream,
    ) -> StorageResult<UploadResponse> {
        let path_str = path.display().to_string();

        // Create writer with OpenDAL's native API
        let mut writer = self.opendal_impl.writer(&path_str).await?;

        // Stream data directly to the writer using native write method
        let mut stream = Box::pin(stream);
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| StorageError::Any(Box::new(e)))?;
            // Use the native write method which handles the data more efficiently
            writer.write(chunk).await?;
        }

        let meta = writer.close().await?;

        Ok(UploadResponse {
            e_tag: meta.etag().map(std::string::ToString::to_string),
            version: meta.version().map(std::string::ToString::to_string),
        })
    }
}
