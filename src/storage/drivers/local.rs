use std::{
    io,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use bytes::Bytes;
use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;

use super::{normalize, not_found, GetResponse, ListEntry, StoreDriver, UploadResponse};
use crate::storage::{stream::BytesStream, StorageError, StorageResult};

/// Filesystem storage on `tokio::fs`, rooted at a directory.
pub struct LocalStore {
    root: PathBuf,
}

/// Create new filesystem storage rooted at the current working directory.
///
/// Keys are resolved relative to the process working directory, so a key such
/// as `uploads/avatar.png` lands under the app directory rather than at the
/// filesystem root. To root the store elsewhere (including an absolute path),
/// use [`new_with_prefix`].
///
/// # Examples
///```
/// use loco_rs::storage::drivers::local;
/// let file_system_driver = local::new();
/// ```
#[must_use]
pub fn new() -> Box<dyn StoreDriver> {
    Box::new(LocalStore {
        root: PathBuf::from("."),
    })
}

/// Create new filesystem storage with `prefix` applied to all paths
///
/// # Examples
///```
/// use loco_rs::storage::drivers::local;
/// let file_system_driver = local::new_with_prefix("users");
/// ```
///
/// # Errors
///
/// Returns an error if the root directory cannot be created
pub fn new_with_prefix(prefix: impl AsRef<Path>) -> StorageResult<Box<dyn StoreDriver>> {
    let root = prefix.as_ref().to_path_buf();
    std::fs::create_dir_all(&root)?;
    Ok(Box::new(LocalStore { root }))
}

impl LocalStore {
    /// The file a key maps to. A key naming a directory (trailing `/`) is
    /// only valid where a directory is.
    fn file(&self, path: &Path) -> StorageResult<(String, PathBuf)> {
        let key = normalize(path);
        if key.is_empty() || key.ends_with('/') {
            return Err(io::Error::new(io::ErrorKind::IsADirectory, key).into());
        }
        let full = self.root.join(&key);
        Ok((key, full))
    }

    async fn create_parent(full: &Path) -> io::Result<()> {
        match full.parent() {
            Some(parent) => tokio::fs::create_dir_all(parent).await,
            None => Ok(()),
        }
    }

    /// Metadata for `key`. A key ending in `/` names only a directory; a
    /// directory also answers to its name without the slash.
    async fn metadata(&self, key: &str) -> io::Result<std::fs::Metadata> {
        let meta = tokio::fs::metadata(self.root.join(key)).await?;
        if key.ends_with('/') && !meta.is_dir() {
            return Err(not_found(key));
        }
        Ok(meta)
    }
}

fn entry(path: String, meta: &std::fs::Metadata) -> ListEntry {
    ListEntry::new(
        path,
        meta.is_dir(),
        Some(meta.len()),
        meta.modified()
            .ok()
            .map(chrono::DateTime::<chrono::Utc>::from),
        None,
    )
}

#[async_trait]
impl StoreDriver for LocalStore {
    async fn upload(&self, path: &Path, content: &Bytes) -> StorageResult<UploadResponse> {
        let (_, full) = self.file(path)?;
        Self::create_parent(&full).await?;
        tokio::fs::write(&full, content).await?;
        Ok(UploadResponse::new(None, None))
    }

    async fn get(&self, path: &Path) -> StorageResult<GetResponse> {
        let (_, full) = self.file(path)?;
        Ok(GetResponse::from_bytes(
            tokio::fs::read(&full).await?.into(),
        ))
    }

    async fn delete(&self, path: &Path) -> StorageResult<()> {
        let key = normalize(path);
        let full = self.root.join(&key);
        let result = if key.is_empty() || key.ends_with('/') {
            tokio::fs::remove_dir(&full).await
        } else {
            tokio::fs::remove_file(&full).await
        };
        match result {
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
            other => Ok(other?),
        }
    }

    async fn rename(&self, from: &Path, to: &Path) -> StorageResult<()> {
        let (_, from) = self.file(from)?;
        let (_, to) = self.file(to)?;
        tokio::fs::metadata(&from).await?;
        Self::create_parent(&to).await?;
        Ok(tokio::fs::rename(&from, &to).await?)
    }

    async fn copy(&self, from: &Path, to: &Path) -> StorageResult<()> {
        let (_, from) = self.file(from)?;
        let (_, to) = self.file(to)?;
        tokio::fs::metadata(&from).await?;
        Self::create_parent(&to).await?;
        tokio::fs::copy(&from, &to).await?;
        Ok(())
    }

    async fn exists(&self, path: &Path) -> StorageResult<bool> {
        Ok(self.metadata(&normalize(path)).await.is_ok())
    }

    async fn list(&self, path: &Path, recursive: bool) -> StorageResult<Vec<ListEntry>> {
        let mut prefix = normalize(path);
        if !prefix.is_empty() && !prefix.ends_with('/') {
            prefix.push('/');
        }
        let mut entries = Vec::new();
        let mut pending = vec![prefix.clone()];
        while let Some(dir) = pending.pop() {
            let mut read = match tokio::fs::read_dir(self.root.join(&dir)).await {
                Ok(read) => read,
                Err(err)
                    if matches!(
                        err.kind(),
                        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
                    ) =>
                {
                    continue;
                }
                Err(err) => return Err(err.into()),
            };
            // A listed directory includes itself, the root spelled `/`.
            if dir == prefix {
                let own = if dir.is_empty() {
                    "/".to_string()
                } else {
                    dir.clone()
                };
                entries.push(ListEntry::new(own, true, Some(0), None, None));
            }
            while let Some(child) = read.next_entry().await? {
                let meta = child.metadata().await?;
                let mut key = format!("{dir}{}", child.file_name().to_string_lossy());
                if meta.is_dir() {
                    key.push('/');
                    if recursive {
                        pending.push(key.clone());
                    }
                }
                entries.push(entry(key, &meta));
            }
        }
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(entries)
    }

    async fn stat(&self, path: &Path) -> StorageResult<ListEntry> {
        let meta = self.metadata(&normalize(path)).await?;
        Ok(entry(path.display().to_string(), &meta))
    }

    async fn get_stream(&self, path: &Path) -> StorageResult<BytesStream> {
        let (_, full) = self.file(path)?;
        let file = tokio::fs::File::open(&full).await?;
        Ok(BytesStream::from_body_stream(
            tokio_util::io::ReaderStream::new(file),
        ))
    }

    async fn upload_stream(
        &self,
        path: &Path,
        stream: BytesStream,
    ) -> StorageResult<UploadResponse> {
        let (_, full) = self.file(path)?;
        Self::create_parent(&full).await?;
        let mut file = tokio::fs::File::create(&full).await?;
        let mut stream = Box::pin(stream);
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| StorageError::Any(Box::new(e)))?;
            file.write_all(&chunk).await?;
        }
        file.flush().await?;
        Ok(UploadResponse::new(None, None))
    }
}
