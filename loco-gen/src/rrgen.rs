//! Renders generator templates and writes them into an app.
//!
//! A template is YAML frontmatter, a `---` line, then the body. The whole
//! template is rendered with Tera first, then the frontmatter says where the
//! body goes (`to`), when to skip it, and which other files to inject into.
//!
//! This was the `rrgen` crate. It lives here so generators render with the
//! same Tera 2 that `loco-rs` already builds; `rrgen` pulled in Tera 1 and
//! everything Tera 1 depends on, the largest single cost in an app's build.
use std::path::{Path, PathBuf};

use chrono::DateTime;
use cruet::string::pluralize;
use heck::{ToKebabCase, ToLowerCamelCase, ToSnakeCase, ToTitleCase, ToUpperCamelCase};
use regex::Regex;
use serde::{Deserialize, Deserializer};
use tera::{Context, Kwargs, State, Tera, TeraResult};

#[derive(Deserialize, Debug, Default)]
struct FrontMatter {
    to: String,

    #[serde(default)]
    skip_exists: bool,

    #[serde(default)]
    skip_glob: Option<String>,

    #[serde(default)]
    message: Option<String>,

    #[serde(default)]
    injections: Option<Vec<Injection>>,
}

#[derive(Deserialize, Debug, Default)]
struct Injection {
    into: String,
    content: String,

    #[serde(default, deserialize_with = "regex")]
    skip_if: Option<Regex>,

    #[serde(default, deserialize_with = "regex")]
    before: Option<Regex>,

    #[serde(default, deserialize_with = "regex")]
    before_last: Option<Regex>,

    #[serde(default, deserialize_with = "regex")]
    after: Option<Regex>,

    #[serde(default, deserialize_with = "regex")]
    after_last: Option<Regex>,

    #[serde(default, deserialize_with = "regex")]
    remove_lines: Option<Regex>,

    #[serde(default)]
    prepend: bool,

    #[serde(default)]
    append: bool,
}

fn regex<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<Regex>, D::Error> {
    Option::<String>::deserialize(deserializer)?
        .map(|pattern| Regex::new(&pattern).map_err(serde::de::Error::custom))
        .transpose()
}

#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error("{0}")]
    Message(String),

    #[error("cannot inject into `{path}`: file does not exist")]
    InjectionTargetMissing { path: String },

    #[error(
        "cannot inject into `{path}`: no line matches the `{strategy}` pattern `{pattern}`\n\n\
         Nothing was written, so this was not added:\n\n{content}\n\n\
         Restore a line matching that pattern in `{path}` and run this again, or \
         add the content by hand."
    )]
    InjectionAnchorNotFound {
        path: String,
        strategy: &'static str,
        pattern: String,
        content: String,
    },

    #[error(
        "cannot inject into `{path}`: the injection says where to write but not where to put it \
         — expected one of `before`, `before_last`, `after`, `after_last`, `prepend`, `append`, \
         or `remove_lines`"
    )]
    InjectionHasNoPlacement { path: String },

    #[error(transparent)]
    Tera(#[from] tera::Error),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    #[allow(clippy::upper_case_acronyms)]
    YAML(#[from] serde_yaml::Error),
    #[error(transparent)]
    Glob(#[from] glob::PatternError),
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum GenResult {
    Skipped,
    Generated { message: Option<String> },
}

fn apply_injection(injection: &Injection, file_content: &str) -> Result<Option<String>> {
    let content = &injection.content;

    let new_content = if injection.prepend {
        format!("{content}\n{file_content}")
    } else if injection.append {
        format!("{file_content}\n{content}")
    } else if let Some(before) = &injection.before {
        insert(injection, file_content, "before", |lines| {
            lines.iter().position(|ln| before.is_match(ln))
        })?
    } else if let Some(before_last) = &injection.before_last {
        insert(injection, file_content, "before_last", |lines| {
            lines.iter().rposition(|ln| before_last.is_match(ln))
        })?
    } else if let Some(after) = &injection.after {
        insert(injection, file_content, "after", |lines| {
            lines
                .iter()
                .position(|ln| after.is_match(ln))
                .map(|p| p + 1)
        })?
    } else if let Some(after_last) = &injection.after_last {
        insert(injection, file_content, "after_last", |lines| {
            lines
                .iter()
                .rposition(|ln| after_last.is_match(ln))
                .map(|p| p + 1)
        })?
    } else if let Some(remove_lines) = &injection.remove_lines {
        let kept = file_content
            .lines()
            .filter(|line| !remove_lines.is_match(line))
            .collect::<Vec<_>>();
        if kept.len() == file_content.lines().count() {
            return Ok(None);
        }
        kept.join("\n")
    } else {
        return Err(Error::InjectionHasNoPlacement {
            path: injection.into.clone(),
        });
    };

    Ok(Some(keep_trailing_newline(file_content, new_content)))
}

fn insert(
    injection: &Injection,
    file_content: &str,
    strategy: &'static str,
    locate: impl Fn(&[&str]) -> Option<usize>,
) -> Result<String> {
    let mut lines = file_content.lines().collect::<Vec<_>>();
    let pos = locate(&lines).ok_or_else(|| Error::InjectionAnchorNotFound {
        path: injection.into.clone(),
        strategy,
        pattern: injection.pattern(strategy),
        content: indent(&injection.content),
    })?;
    lines.insert(pos, &injection.content);
    Ok(lines.join("\n"))
}

fn keep_trailing_newline(original: &str, mut new: String) -> String {
    if original.ends_with('\n') && !new.ends_with('\n') {
        new.push('\n');
    }
    new
}

fn indent(content: &str) -> String {
    content
        .lines()
        .map(|line| format!("    {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

impl Injection {
    fn pattern(&self, strategy: &str) -> String {
        let pattern = match strategy {
            "before" => self.before.as_ref(),
            "before_last" => self.before_last.as_ref(),
            "after" => self.after.as_ref(),
            "after_last" => self.after_last.as_ref(),
            _ => None,
        };
        pattern.map_or_else(|| "<none>".to_string(), ToString::to_string)
    }
}

fn parse_template(input: &str) -> Result<(FrontMatter, String)> {
    // normalize line endings
    let input = input.replace("\r\n", "\n");

    let (fm, body) = input.split_once("---\n").ok_or_else(|| {
        Error::Message("cannot split document to frontmatter and body".to_string())
    })?;
    let frontmatter: FrontMatter = serde_yaml::from_str(fm)?;
    Ok((frontmatter, body.to_string()))
}

/// `{{ ts | date(format="%Y%m%d_%H%M%S") }}` on an RFC 3339 timestamp. Tera 1
/// had this built in; Tera 2 does not.
// `Kwargs` by value is Tera's `Filter` signature, not a choice.
#[allow(clippy::needless_pass_by_value)]
fn date(value: &str, kwargs: Kwargs, _: &State<'_>) -> TeraResult<String> {
    let format: String = kwargs
        .get("format")?
        .unwrap_or_else(|| "%Y-%m-%d".to_string());
    let ts = DateTime::parse_from_rfc3339(value).map_err(|err| {
        tera::Error::message(format!("`date` expects an RFC 3339 timestamp: {err}"))
    })?;
    Ok(ts.format(&format).to_string())
}

fn template_engine() -> Tera {
    let mut tera = Tera::default();
    tera.register_filter("pascal_case", |s: &str, _: Kwargs, _: &State<'_>| {
        s.to_upper_camel_case()
    });
    tera.register_filter("camel_case", |s: &str, _: Kwargs, _: &State<'_>| {
        s.to_lower_camel_case()
    });
    tera.register_filter("kebab_case", |s: &str, _: Kwargs, _: &State<'_>| {
        s.to_kebab_case()
    });
    tera.register_filter("lower_case", |s: &str, _: Kwargs, _: &State<'_>| {
        s.to_lowercase()
    });
    tera.register_filter("snake_case", |s: &str, _: Kwargs, _: &State<'_>| {
        s.to_snake_case()
    });
    tera.register_filter("title_case", |s: &str, _: Kwargs, _: &State<'_>| {
        s.to_title_case()
    });
    tera.register_filter("upper_case", |s: &str, _: Kwargs, _: &State<'_>| {
        s.to_uppercase()
    });
    tera.register_filter("plural", |s: &str, _: Kwargs, _: &State<'_>| {
        pluralize::to_plural(s)
    });
    tera.register_filter("date", date);
    tera
}

pub struct RRgen {
    working_dir: Option<PathBuf>,
    template_engine: Tera,
}

impl Default for RRgen {
    fn default() -> Self {
        Self {
            working_dir: None,
            template_engine: template_engine(),
        }
    }
}

impl RRgen {
    #[must_use]
    pub fn with_working_dir<P: AsRef<Path>>(path: P) -> Self {
        Self {
            working_dir: Some(path.as_ref().to_path_buf()),
            ..Default::default()
        }
    }

    /// Renders `input` with `vars` and writes the result where its frontmatter
    /// says.
    ///
    /// # Errors
    ///
    /// When the template does not render or parse, an injection target or
    /// anchor is missing, or a file cannot be written.
    pub fn generate(&self, input: &str, vars: &serde_json::Value) -> Result<GenResult> {
        let rendered =
            self.template_engine
                .render_str(input, &Context::from_serialize(vars)?, false)?;
        let (frontmatter, body) = parse_template(&rendered)?;

        let path_to = if let Some(working_dir) = &self.working_dir {
            working_dir.join(frontmatter.to)
        } else {
            PathBuf::from(&frontmatter.to)
        };

        if frontmatter.skip_exists && path_to.exists() {
            println!("skipped (exists): {}", path_to.display());
            return Ok(GenResult::Skipped);
        }
        if let Some(skip_glob) = frontmatter.skip_glob {
            // Resolve against the working dir, like every other path in the
            // template. Globbing the process's current directory instead means
            // the skip silently never fires for a caller that set one.
            let skip_glob = self.working_dir.as_ref().map_or_else(
                || skip_glob.clone(),
                |dir| dir.join(&skip_glob).to_string_lossy().into_owned(),
            );
            if glob::glob(&skip_glob)?.count() > 0 {
                println!("skipped (exists): {}", path_to.display());
                return Ok(GenResult::Skipped);
            }
        }

        // Work every injection out before writing anything.
        //
        // An injection can fail, and failing after the main file is written
        // leaves the caller stuck: the file exists, so a second run hits
        // `skip_exists`/`skip_glob` and returns before it reaches the injection
        // that never happened. Deciding everything first makes a failed
        // generation a no-op you can simply re-run.
        let pending = self.plan_injections(frontmatter.injections.as_deref())?;

        if path_to.exists() {
            println!("overwritten: {}", path_to.display());
        } else {
            println!("added: {}", path_to.display());
        }
        write_file(&path_to, &body)?;

        for (path, content) in pending {
            write_file(&path, &content)?;
            println!("injected: {}", path.display());
        }

        Ok(GenResult::Generated {
            message: frontmatter.message.clone(),
        })
    }

    fn plan_injections(&self, injections: Option<&[Injection]>) -> Result<Vec<(PathBuf, String)>> {
        let mut pending: Vec<(PathBuf, String)> = Vec::new();

        for injection in injections.unwrap_or_default() {
            let injection_to = self.working_dir.as_ref().map_or_else(
                || PathBuf::from(&injection.into),
                |working_dir| working_dir.join(&injection.into),
            );
            if !injection_to.exists() {
                return Err(Error::InjectionTargetMissing {
                    path: injection.into.clone(),
                });
            }

            // Later injections see earlier ones. Two injections into the same
            // file is ordinary — an import line and a registration line — and
            // computing both against the on-disk copy would lose the first.
            let file_content = match pending.iter().rev().find(|(path, _)| path == &injection_to) {
                Some((_, planned)) => planned.clone(),
                None => std::fs::read_to_string(&injection_to)?,
            };

            if let Some(skip_if) = &injection.skip_if
                && skip_if.is_match(&file_content)
            {
                continue;
            }

            let Some(new_content) = apply_injection(injection, &file_content)? else {
                // A removal that matched nothing. The file is already in the
                // state the template asked for, so there is nothing to write
                // and nothing to report.
                continue;
            };

            pending.push((injection_to, new_content));
        }

        Ok(pending)
    }
}

fn write_file(path: &Path, content: &str) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    Ok(std::fs::write(path, content)?)
}
