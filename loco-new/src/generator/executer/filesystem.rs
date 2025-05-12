use std::path::{Path, PathBuf};

use fs_extra::file::{move_file, write_all};
use walkdir::WalkDir;

use super::Executer;
use crate::{generator, settings::Settings};

#[derive(Debug, Default, Clone)]
pub struct FileSystem {
    pub source_dir: PathBuf,
    pub target_dir: PathBuf,
    pub template_engine: generator::template::Template,
}

impl FileSystem {
    #[must_use]
    pub fn new(from: &Path, to: &Path) -> Self {
        Self {
            source_dir: from.to_path_buf(),
            target_dir: to.to_path_buf(),
            template_engine: generator::template::Template::default(),
        }
    }

    #[must_use]
    pub fn with_template_engine(
        from: &Path,
        to: &Path,
        template_engine: generator::template::Template,
    ) -> Self {
        Self {
            source_dir: from.to_path_buf(),
            target_dir: to.to_path_buf(),
            template_engine,
        }
    }

    fn render_and_rename_template_file(
        &self,
        file_path: &Path,
        settings: &Settings,
    ) -> super::Result<()> {
        let template_content = fs_extra::file::read_to_string(file_path).map_err(|err| {
            tracing::debug!(err = %err, "failed to read template file");
            err
        })?;
        let rendered_content = self.template_engine.render(&template_content, settings)?;
        write_all(file_path, &rendered_content).map_err(|err| {
            tracing::debug!(err = %err, "failed to write rendered content to file");
            err
        })?;

        let renamed_path = self
            .template_engine
            .strip_template_extension(file_path)
            .map_err(|err| {
                tracing::debug!(err = %err, "error stripping template extension from file");
                super::Error::msg("error striping template file")
            })?;
        move_file(file_path, renamed_path, &fs_extra::file::CopyOptions::new())?;
        Ok(())
    }
}

impl Executer for FileSystem {
    fn copy_file(&self, path: &Path) -> super::Result<PathBuf> {
        let source_path = self.source_dir.join(path);
        let target_path = self.target_dir.join(path);

        let span = tracing::error_span!("copy_file", source_path = %source_path.display(), target_path = %target_path.display());
        let _guard = span.enter();

        tracing::debug!("starting file copy operation");

        fs_extra::dir::create_all(target_path.parent().unwrap(), false).map_err(|error| {
            tracing::debug!(error = %error, "error creating target parent directory");
            error
        })?;

        let copy_options = fs_extra::file::CopyOptions::new();
        fs_extra::file::copy(source_path, &target_path, &copy_options)?;
        tracing::debug!("file copy completed successfully");

        Ok(target_path)
    }

    fn create_file(&self, path: &Path, content: String) -> super::Result<PathBuf> {
        let target_path = self.target_dir.join(path);
        if let Some(parent) = path.parent() {
            fs_extra::dir::create_all(self.target_dir.join(parent), false)?;
        }

        let span = tracing::info_span!("create_file", target_path = %target_path.display());
        let _guard = span.enter();

        tracing::debug!("starting file copy operation");

        fs_extra::dir::create_all(target_path.parent().unwrap(), false).map_err(|error| {
            tracing::debug!(error = %error, "error creating target parent directory");
            error
        })?;

        fs_extra::file::write_all(&target_path, &content)?;
        tracing::debug!("file created successfully");

        Ok(target_path)
    }

    fn copy_dir(&self, directory_path: &Path) -> super::Result<()> {
        let source_path = self.source_dir.join(directory_path);
        let target_path = self.target_dir.join(directory_path);

        let span = tracing::error_span!("", source_path = %source_path.display(), target_path = %target_path.display());
        let _guard = span.enter();

        tracing::debug!("starting directory copy operation");
        let copy_options = fs_extra::dir::CopyOptions::new().copy_inside(true);
        fs_extra::dir::copy(source_path, target_path, &copy_options)?;
        tracing::debug!("directory copy completed successfully");
        Ok(())
    }

    fn copy_template(&self, file_path: &Path, settings: &Settings) -> super::Result<()> {
        let span = tracing::error_span!("copy_template", file_path = %file_path.display());
        let _guard: tracing::span::Entered<'_> = span.enter();
        if !self.template_engine.is_template(file_path) {
            tracing::debug!("file is not a template, skipping rendering");
            return Err(super::Error::msg("File is not a template"));
        }

        //todo fix the if here
        tracing::debug!("copying template file");

        let copied_path = self.copy_file(file_path)?;
        self.render_and_rename_template_file(&copied_path, settings)
    }

    #[allow(clippy::cognitive_complexity)]
    fn copy_template_dir(&self, directory_path: &Path, settings: &Settings) -> super::Result<()> {
        let source_path = self.source_dir.join(directory_path);
        let target_path = self.target_dir.join(directory_path);

        let span = tracing::error_span!("copy_template_dir", source_path = %source_path.display(), target_path = %target_path.display());
        let _guard: tracing::span::Entered<'_> = span.enter();

        tracing::debug!("starting template directory copy operation");

        let copy_options = fs_extra::dir::CopyOptions::new().copy_inside(true);
        fs_extra::dir::copy(source_path, target_path, &copy_options)?;

        tracing::debug!("scanning copied directory for template files to render");
        for entry in WalkDir::new(self.target_dir.join(directory_path))
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if self.template_engine.is_template(path) {
                tracing::debug!(template_path = %path.display(), "rendering template file in directory");
                self.render_and_rename_template_file(path, settings)?;
            } else {
                tracing::debug!(file_path = %path.display(), "not a template file");
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tree_fs::TreeBuilder;

    use super::*;

    fn init_filesystem() -> (FileSystem, tree_fs::Tree) {
        let tree_fs = TreeBuilder::default()
            .add("test/foo.txt", "bar")
            .add("test/bar.txt.t", "crate: {{settings.package_name}}")
            .create()
            .expect("Failed to create mock data");

        let copy_to = TreeBuilder::default()
            .create()
            .expect("Failed to create mock data");
        (FileSystem::new(&tree_fs.root, &copy_to.root), tree_fs)
    }

    #[test]
    fn can_copy_file() {
        let (fs, _tree_fs) = init_filesystem();

        assert!(fs.copy_file(&PathBuf::from("test").join("foo.txt")).is_ok());
        let copied_path = fs.target_dir.join("test").join("foo.txt");
        assert!(copied_path.exists());
        assert_eq!(
            fs_extra::file::read_to_string(copied_path).expect("read content"),
            "bar"
        );
    }

    #[test]
    fn can_copy_dir() {
        let (fs, _tree_fs) = init_filesystem();
        assert!(fs.copy_dir(&PathBuf::from("test")).is_ok());
        let copied_path_1 = fs.target_dir.join("test").join("foo.txt");
        let copied_path_2 = fs.target_dir.join("test").join("bar.txt.t");
        assert!(copied_path_1.exists());
        assert!(copied_path_2.exists());

        assert_eq!(
            fs_extra::file::read_to_string(copied_path_1).expect("read content"),
            "bar"
        );

        assert_eq!(
            fs_extra::file::read_to_string(copied_path_2).expect("read content"),
            "crate: {{settings.package_name}}"
        );
    }

    #[test]
    fn can_copy_template() {
        let (fs, _tree_fs) = init_filesystem();

        let settings = Settings {
            package_name: "loco-app".to_string(),
            ..Default::default()
        };

        assert!(fs
            .copy_template(&PathBuf::from("test").join("bar.txt.t"), &settings)
            .is_ok());
        let copied_path = fs.target_dir.join("test").join("bar.txt");
        assert!(copied_path.exists());
        assert_eq!(
            fs_extra::file::read_to_string(copied_path).expect("read content"),
            "crate: loco-app"
        );
    }

    #[test]
    fn can_copy_template_dir() {
        let (fs, _tree_fs) = init_filesystem();

        let settings = Settings {
            package_name: "loco-app".to_string(),
            ..Default::default()
        };

        assert!(fs
            .copy_template_dir(&PathBuf::from("test"), &settings)
            .is_ok());
        let copied_path_1 = fs.target_dir.join("test").join("foo.txt");
        let copied_path_2 = fs.target_dir.join("test").join("bar.txt");
        assert!(copied_path_1.exists());
        assert!(copied_path_2.exists());

        assert_eq!(
            fs_extra::file::read_to_string(copied_path_1).expect("read content"),
            "bar"
        );

        assert_eq!(
            fs_extra::file::read_to_string(copied_path_2).expect("read content"),
            "crate: loco-app"
        );
    }
}
