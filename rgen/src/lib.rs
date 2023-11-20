use std::path::Path;

use gray_matter::{engine::YAML, Matter};
use regex::Regex;
use serde::Deserialize;
use tera::{Context, Tera};

mod tera_filters;
pub trait FsDriver {
    fn write_file(&self, path: &Path, content: &str) -> Result<()>;
    fn read_file(&self, path: &Path) -> Result<String>;
    fn exists(&self, path: &Path) -> bool;
}

pub struct RealFsDriver {}
impl FsDriver for RealFsDriver {
    fn write_file(&self, path: &Path, content: &str) -> Result<()> {
        Ok(fs_err::write(path, content)?)
    }

    fn read_file(&self, path: &Path) -> Result<String> {
        Ok(fs_err::read_to_string(path)?)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
}

pub trait Printer {
    fn overwrite_file(&self, file_to: &Path);
    fn add_file(&self, file_to: &Path);
    fn injected(&self, file_to: &Path);
}
pub struct ConsolePrinter {}
impl Printer for ConsolePrinter {
    fn overwrite_file(&self, file_to: &Path) {
        println!("overwritten: {file_to:?}");
    }

    fn add_file(&self, file_to: &Path) {
        println!("added: {file_to:?}");
    }

    fn injected(&self, file_to: &Path) {
        println!("injected: {file_to:?}");
    }
}

#[derive(Deserialize, Debug, Default)]
struct FrontMatter {
    to: String,

    #[serde(default)]
    injections: Option<Vec<Injection>>,
}

#[derive(Deserialize, Debug, Default)]
struct Injection {
    into: String,
    content: String,

    #[serde(with = "serde_regex")]
    #[serde(default)]
    skip_if: Option<Regex>,

    #[serde(with = "serde_regex")]
    #[serde(default)]
    before: Option<Regex>,

    #[serde(with = "serde_regex")]
    #[serde(default)]
    after: Option<Regex>,

    #[serde(default)]
    prepend: bool,

    #[serde(default)]
    append: bool,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Tera(#[from] tera::Error),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error(transparent)]
    Any(Box<dyn std::error::Error + Send + Sync>),
}
type Result<T> = std::result::Result<T, Error>;

pub struct Rgen {
    fs: Box<dyn FsDriver>,
    printer: Box<dyn Printer>,
}

impl Default for Rgen {
    fn default() -> Self {
        Self {
            fs: Box::new(RealFsDriver {}),
            printer: Box::new(ConsolePrinter {}),
        }
    }
}

impl Rgen {
    pub fn generate(&self, input: &str, vars: &serde_json::Value) -> Result<()> {
        let mut tera = Tera::default();
        tera_filters::register_all(&mut tera);
        let rendered = tera.render_str(input, &Context::from_serialize(vars.clone())?)?;
        let matter = Matter::<YAML>::new();
        let parsed = matter.parse(&rendered);
        let fmatter: FrontMatter = parsed
            .data
            .ok_or_else(|| Error::Message("cannot find frontmatter".to_string()))?
            .deserialize()?;
        let path_to = Path::new(&fmatter.to);

        // write main file
        self.fs.write_file(path_to, &parsed.content)?;
        if self.fs.exists(path_to) {
            self.printer.overwrite_file(path_to);
        } else {
            self.printer.add_file(path_to);
        }

        // handle injects
        if let Some(injections) = fmatter.injections {
            for injection in &injections {
                let injection_to = Path::new(&injection.into);
                if !self.fs.exists(injection_to) {
                    return Err(Error::Message(format!(
                        "cannot inject into {}: file does not exist",
                        injection.into,
                    )));
                }

                let file_content = self.fs.read_file(injection_to)?;
                let content = &injection.content;

                if let Some(skip_if) = &injection.skip_if {
                    if skip_if.is_match(&file_content) {
                        continue;
                    }
                }

                let new_content = if injection.prepend {
                    format!("{content}\n{file_content}")
                } else if injection.append {
                    format!("{file_content}\n{content}")
                } else if let Some(before) = &injection.before {
                    let mut lines = file_content.lines().collect::<Vec<_>>();
                    let pos = lines.iter().position(|ln| before.is_match(ln));
                    if let Some(pos) = pos {
                        lines.insert(pos, content);
                    }
                    lines.join("\n")
                } else if let Some(after) = &injection.after {
                    let mut lines = file_content.lines().collect::<Vec<_>>();
                    let pos = lines.iter().position(|ln| after.is_match(ln));
                    if let Some(pos) = pos {
                        lines.insert(pos + 1, content);
                    }
                    lines.join("\n")
                } else {
                    println!("warning: no injection made");
                    file_content.clone()
                };

                self.fs.write_file(injection_to, &new_content)?;
                self.printer.injected(injection_to);
            }
        }
        //text: Tera::one_off(text, &Context::from_serialize(args.clone())?,
        // true)?, render entire file
        // parse frontmatter
        // deserialize frontmatter into struct
        // operate on struct and fs
        // 1. check templating (tera)
        // 2. test frontmatter (graymatter)
        // 3. build trait for fs
        // 4. write out logic
        Ok(())
    }
}
