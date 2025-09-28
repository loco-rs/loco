use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use opendal::Reader;

/// A stream of bytes that abstracts over the underlying storage implementation.
/// This type ensures that `OpenDAL` types are not exposed in the public API.
pub struct BytesStream {
    inner: Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>>,
}

impl BytesStream {
    /// Create a `BytesStream` from an `OpenDAL` `Reader`.
    /// This is an internal method used by storage drivers.
    pub(crate) async fn from_reader(reader: Reader) -> Result<Self, crate::storage::StorageError> {
        // Convert the Reader into a stream of bytes
        // The range parameter (..) means we want to read the entire content
        let stream = reader
            .into_bytes_stream(..)
            .await
            .map_err(crate::storage::StorageError::from)?;

        // Convert opendal::Error to std::io::Error for uniform error handling
        let mapped_stream = stream
            .map(|result| result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)));

        Ok(Self {
            inner: Box::pin(mapped_stream),
        })
    }

    /// Collect the entire stream into a single `Bytes` buffer.
    /// This method should be used carefully as it loads the entire content into memory.
    ///
    /// # Errors
    ///
    /// Returns an error if reading from the stream fails.
    pub async fn collect(mut self) -> Result<Bytes, std::io::Error> {
        let mut buffer = Vec::new();

        while let Some(chunk) = self.next().await {
            let chunk = chunk?;
            buffer.extend_from_slice(&chunk);
        }

        Ok(Bytes::from(buffer))
    }
}

impl Stream for BytesStream {
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

/// Extension trait for axum integration
impl BytesStream {
    /// Convert the `BytesStream` into an axum `Body` for HTTP responses.
    /// This enables zero-copy streaming directly to the HTTP response.
    #[must_use]
    pub fn into_body(self) -> axum::body::Body {
        axum::body::Body::from_stream(self)
    }

    /// Create a `BytesStream` from an axum request body.
    /// This is useful for streaming uploads.
    ///
    /// Note: This requires the body to be converted to a stream first.
    /// In practice, you might want to use axum's extractors directly.
    pub fn from_body_stream<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<Bytes, std::io::Error>> + Send + 'static,
    {
        Self {
            inner: Box::pin(stream),
        }
    }
}
