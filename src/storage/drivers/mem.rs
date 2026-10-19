use std::{collections::BTreeMap, path::Path, sync::Mutex};

use async_trait::async_trait;
use bytes::Bytes;
use chrono::{DateTime, Utc};

use super::{normalize, not_found, GetResponse, ListEntry, StoreDriver, UploadResponse};
use crate::storage::StorageResult;

/// In-memory storage: a sorted map from key to content.
#[derive(Default)]
pub struct MemStore {
    files: Mutex<BTreeMap<String, (Bytes, DateTime<Utc>)>>,
}

/// Create new in-memory storage.
///
/// # Examples
///```
/// use loco_rs::storage::drivers::mem;
/// let mem_storage = mem::new();
/// ```
#[must_use]
pub fn new() -> Box<dyn StoreDriver> {
    Box::new(MemStore::default())
}

impl MemStore {
    fn files(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, (Bytes, DateTime<Utc>)>> {
        self.files
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn key(path: &Path) -> StorageResult<String> {
        let key = normalize(path);
        if key.is_empty() || key.ends_with('/') {
            return Err(std::io::Error::new(std::io::ErrorKind::IsADirectory, key).into());
        }
        Ok(key)
    }

    fn read(&self, path: &Path) -> StorageResult<Bytes> {
        let key = Self::key(path)?;
        let content = self.files().get(&key).map(|(content, _)| content.clone());
        Ok(content.ok_or_else(|| not_found(&key))?)
    }
}

#[async_trait]
impl StoreDriver for MemStore {
    async fn upload(&self, path: &Path, content: &Bytes) -> StorageResult<UploadResponse> {
        let key = Self::key(path)?;
        self.files().insert(key, (content.clone(), Utc::now()));
        Ok(UploadResponse::new(None, None))
    }

    async fn get(&self, path: &Path) -> StorageResult<GetResponse> {
        Ok(GetResponse::from_bytes(self.read(path)?))
    }

    async fn delete(&self, path: &Path) -> StorageResult<()> {
        self.files().remove(&normalize(path));
        Ok(())
    }

    async fn rename(&self, from: &Path, to: &Path) -> StorageResult<()> {
        let (from, to) = (Self::key(from)?, Self::key(to)?);
        let mut files = self.files();
        let file = files.remove(&from).ok_or_else(|| not_found(&from))?;
        files.insert(to, file);
        drop(files);
        Ok(())
    }

    async fn copy(&self, from: &Path, to: &Path) -> StorageResult<()> {
        let content = self.read(from)?;
        self.upload(to, &content).await?;
        Ok(())
    }

    async fn exists(&self, path: &Path) -> StorageResult<bool> {
        Ok(self.stat(path).await.is_ok())
    }

    async fn list(&self, path: &Path, recursive: bool) -> StorageResult<Vec<ListEntry>> {
        let mut prefix = normalize(path);
        if !prefix.is_empty() && !prefix.ends_with('/') {
            prefix.push('/');
        }
        let files = self.files();
        let mut entries: Vec<ListEntry> = Vec::new();
        for (key, (content, modified)) in files.range(prefix.clone()..) {
            let Some(rest) = key.strip_prefix(&prefix) else {
                break;
            };
            match rest.split_once('/') {
                Some((dir, _)) if !recursive => {
                    let dir = format!("{prefix}{dir}/");
                    if entries.last().is_none_or(|last| last.path != dir) {
                        entries.push(ListEntry::new(dir, true, Some(0), None, None));
                    }
                }
                _ => entries.push(ListEntry::new(
                    key.clone(),
                    false,
                    Some(content.len() as u64),
                    Some(*modified),
                    None,
                )),
            }
        }
        drop(files);
        Ok(entries)
    }

    async fn stat(&self, path: &Path) -> StorageResult<ListEntry> {
        let key = normalize(path);
        let found = {
            let files = self.files();
            if key.is_empty() || key.ends_with('/') {
                files
                    .keys()
                    .any(|k| k.starts_with(&key))
                    .then_some((true, 0, None))
            } else {
                files
                    .get(&key)
                    .map(|(content, modified)| (false, content.len() as u64, Some(*modified)))
            }
        };
        let (is_dir, size, modified) = found.ok_or_else(|| not_found(&key))?;
        Ok(ListEntry::new(
            path.display().to_string(),
            is_dir,
            Some(size),
            modified,
            None,
        ))
    }
}
