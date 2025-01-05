use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use loco::{
    generator::{self, executer::FileSystem, template},
    settings,
};
use rand::{rngs::StdRng, SeedableRng};


mod auth;
mod background;
mod db;
mod features;
mod initializers;
mod mailer;
mod module_name;
mod rendering_method;

pub struct TestGenerator {
    tree: tree_fs::Tree,
}

impl TestGenerator {
    pub fn generate(settings: settings::Settings) -> Self {
        let tree = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create tree fs");

        let template_engine = template::Template::new(StdRng::seed_from_u64(42));

        let fs: FileSystem = FileSystem::with_template_engine(
            Path::new("base_template"),
            tree.root.as_path(),
            template_engine,
        );

        generator::Generator::new(Arc::new(fs), settings)
            .run()
            .expect("run generate");

        Self { tree }
    }

    pub fn path(&self, path: &str) -> PathBuf {
        self.tree.root.join(path)
    }
}
