use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Mutex,
};

use super::Executer;
use crate::{generator, settings::Settings};

pub struct Inmem {
    pub source_path: PathBuf,
    pub file_store: Mutex<BTreeMap<PathBuf, String>>,
    pub template_engine: generator::template::Template,
}

impl Inmem {
    #[must_use]
    pub fn new(source: &Path) -> Self {
        Self::with_template_engine(source, generator::template::Template::default())
    }

    #[must_use]
    pub fn with_template_engine(
        source: &Path,
        template_engine: generator::template::Template,
    ) -> Self {
        Self {
            source_path: source.to_path_buf(),
            file_store: Mutex::new(BTreeMap::default()),
            template_engine,
        }
    }

    pub fn get_file_content(&self, path: &Path) -> Option<String> {
        self.file_store
            .lock()
            .ok()
            .and_then(|store| store.get(path).cloned())
    }
}

impl Executer for Inmem {
    fn copy_file(&self, file_path: &Path) -> super::Result<PathBuf> {
        let file_content = fs_extra::file::read_to_string(self.source_path.join(file_path))?;
        self.file_store
            .lock()
            .unwrap()
            .insert(file_path.to_path_buf(), file_content);
        Ok(file_path.to_path_buf())
    }

    fn create_file(&self, path: &Path, content: String) -> super::Result<PathBuf> {
        self.file_store
            .lock()
            .unwrap()
            .insert(path.to_path_buf(), content);
        Ok(path.to_path_buf())
    }

    fn copy_dir(&self, directory_path: &Path) -> super::Result<()> {
        let directory_content = fs_extra::dir::get_dir_content(directory_path)?;
        for file in directory_content.files {
            let mut store = self.file_store.lock().unwrap();
            store.insert(PathBuf::from(&file), fs_extra::file::read_to_string(file)?);
        }
        Ok(())
    }

    fn copy_template(&self, file_path: &Path, settings: &Settings) -> super::Result<()> {
        let copied_path = self.copy_file(file_path)?;

        if self.template_engine.is_template(&copied_path) {
            let template_content = {
                let store = self.file_store.lock().unwrap();
                store.get(&copied_path).cloned()
            };

            if let Some(content) = template_content {
                let rendered_content = self.template_engine.render(&content, settings)?;
                self.file_store
                    .lock()
                    .unwrap()
                    .insert(file_path.to_path_buf(), rendered_content);
                Ok(())
            } else {
                Err(super::Error::msg("Template content not found"))
            }
        } else {
            Err(super::Error::msg("File is not a template"))
        }
    }

    fn copy_template_dir(&self, _path: &Path, _data: &Settings) -> super::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tree_fs::{Tree, TreeBuilder};

    use super::*;

    fn init_in_memory_store() -> (Inmem, Tree) {
        let tree = TreeBuilder::default()
            .drop(true)
            .add("test/foo.txt", "bar")
            .add("test/bar.txt.t", "crate: {{settings.package_name}}")
            .create()
            .expect("Failed to create mock data");
        (Inmem::new(&tree.root), tree)
    }

    #[test]
    fn can_copy_file() {
        let (store, source_dir) = init_in_memory_store();
        let test_file_path = source_dir.root.join("test").join("foo.txt");

        let copied_path = store.copy_file(&test_file_path).unwrap();

        assert_eq!(copied_path, test_file_path);
        assert_eq!(
            store
                .file_store
                .lock()
                .unwrap()
                .get(&test_file_path)
                .unwrap(),
            "bar"
        );
    }

    #[test]
    fn test_copy_directory() {
        let (store, source_dir) = init_in_memory_store();
        let dir_path = source_dir.root.join("test");

        store.copy_dir(&dir_path).unwrap();

        let file1_path = dir_path.join("foo.txt");
        let file2_path = dir_path.join("bar.txt.t");

        assert_eq!(
            store.file_store.lock().unwrap().get(&file1_path).unwrap(),
            "bar"
        );
        assert_eq!(
            store.file_store.lock().unwrap().get(&file2_path).unwrap(),
            "crate: {{settings.package_name}}"
        );
    }

    #[test]
    fn can_copy_template_file() {
        let (store, source_dir) = init_in_memory_store();
        let test_file_path = source_dir.root.join("test").join("bar.txt.t");

        let settings = Settings {
            package_name: "loco-app".to_string(),
            ..Default::default()
        };

        store
            .copy_template(&test_file_path, &settings)
            .expect("copy template");

        assert_eq!(
            store
                .file_store
                .lock()
                .unwrap()
                .get(&test_file_path)
                .unwrap(),
            "crate: loco-app"
        );
    }
}
