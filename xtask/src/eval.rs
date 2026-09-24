//! The Loco agent eval: does an agent actually write idiomatic Loco?
//!
//! # Why this shape
//!
//! Two failure modes matter and they need different instruments.
//!
//! **Mechanical gates answer "does it work."** `cargo build`, `clippy -D
//! warnings`, and `cargo test` are objective, free, and unfakeable — an oracle
//! most frameworks' evals do not get. But they are blind to the thing we care
//! about: a hand-rolled retry loop adds no dependency and passes every gate,
//! and so does serialising an entity straight to the wire with the password
//! hash in it.
//!
//! **A model judge answers "is it good."** So gates are *admission control*,
//! not score. Everything they establish — which gates passed, what the
//! dependency delta was, which files were touched — is packed into an evidence
//! packet and handed to the judge as input.
//!
//! # Reference-anchored, always
//!
//! The judge never scores a submission alone. Every task ships a curated
//! reference solution and the judge is asked to compare, in both orders. Absolute
//! 1-10 scoring from a model is not reliable enough to build a KPI series on;
//! pairwise-against-a-fixed-anchor is. That makes the reference corpus
//! load-bearing rather than optional, which is why `--check-reference` exists:
//! a reference that cannot pass its own gates silently invalidates every score
//! derived from it.
//!
//! # Determinism, and the error bar
//!
//! The judge model is pinned. Changing it changes the scale, so a KPI series
//! spanning a model change is not a series.
//!
//! Every arm is judged **twice**, once with the submission above the reference
//! and once below it, and the two passes are averaged. The gap between them is
//! reported as `spread`, and it is the only error bar this eval has: identical
//! code, identical rubric, one presentation difference. A per-arm delta smaller
//! than the mean spread is not a result, and the report says so in those words
//! rather than leaving a reader to infer it.
//!
//! This is a *lower* bound on judge noise. The judge is told which side is the
//! reference, so the swap perturbs order alone — it cannot expose a bias that
//! tracks the label rather than the position.
//!
//! Runs 1-9 predate this: they were scored by a single fixed-order pass, so
//! their idiomatic deltas have no error bar and none can be reconstructed.

use std::{
    collections::BTreeMap,
    // Writing into a `String` is infallible — the `Result` exists only to
    // satisfy the trait — so the prompt/report builders below discard it.
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

use eyre::{bail, Result};
use serde_json::Value;

/// Where tasks live, relative to the project root.
const TASKS_DIR: &str = "evals/tasks";

/// The shared rubric handed to the judge, relative to the project root.
const RUBRIC: &str = "evals/judge/rubric.md";

/// The doctrine the rubric scores against — the same file agents are given.
const DOCTRINE: &str = "skills/loco/doctrine.md";

/// Pinned judge model. Changing this rebases every score; do not change it
/// without restarting the KPI series.
const JUDGE_MODEL: &str = "claude-fable-5";

/// Marks an answer block that extends an existing file instead of replacing it.
const APPEND_PREFIX: &str = "append:";

/// App files shown verbatim to **both** arms, standing in for the `cat` a
/// tool-using agent would run. These are the files every task reads or extends;
/// showing the source rather than a summary of it keeps the harness from
/// curating — and so from leaking — on the agent's behalf.
const APP_CONTEXT: [&str; 2] = ["src/models/_entities/users.rs", "src/models/users.rs"];

/// One eval task, loaded from `evals/tasks/<id>/task.json`.
#[derive(Debug)]
pub struct Task {
    pub id: String,
    pub dir: PathBuf,
    /// One-line description, shown by `--list`.
    pub title: String,
    /// Doctrine principles this task is designed to exercise (`P1`…`P6`).
    pub principles: Vec<String>,
    /// Flags passed to `loco new`. Tasks sharing a signature share a scaffold.
    pub app_flags: Vec<String>,
    /// Files under `skills/loco/` handed to the `with-skill` arm.
    pub skill: Vec<String>,
    /// The files the agent must write. Everything else in `reference/` is
    /// pre-applied wiring, so a one-shot answer can be compiled on its own
    /// without the agent having to rediscover module and route registration.
    pub produce: Vec<String>,
    /// Existing files the agent extends rather than rewrites. Kept separate
    /// from `produce` so a case can exercise the model layer without making
    /// the agent restate a 400-line file it did not write.
    pub append: Vec<String>,
}

impl Task {
    /// The prompt handed to the agent under test.
    fn prompt(&self) -> Result<String> {
        Ok(fs::read_to_string(self.dir.join("PROMPT.md"))?)
    }

    /// The substance of the curated answer: exactly the files the agent is
    /// asked for. Kept apart from `wiring` so the reference can be pre-applied
    /// for compilation without handing over the answer.
    fn reference(&self) -> PathBuf {
        self.dir.join("reference")
    }

    /// Module declarations, route registration and test scaffolding — the
    /// mechanical parts the prompt tells the agent are already done.
    fn wiring(&self) -> PathBuf {
        self.dir.join("wiring")
    }

    /// Snippets appended to existing files, mirroring the agent's `// append:`
    /// blocks, keyed by the file they extend.
    fn appends(&self) -> Result<Vec<(String, String)>> {
        let root = self.dir.join("reference/append");
        let mut out = Vec::new();
        if root.is_dir() {
            let mut files = Vec::new();
            collect_files(&root, &root, &mut files)?;
            files.sort();
            for rel in files {
                out.push((
                    rel.display().to_string(),
                    fs::read_to_string(root.join(&rel))?,
                ));
            }
        }
        Ok(out)
    }
}

/// What one gated run produced.
#[derive(Debug, Default)]
pub struct Outcome {
    pub gates: BTreeMap<String, i32>,
    /// Crates added to `Cargo.toml` beyond what `loco new` produced.
    pub added_deps: Vec<String>,
    /// Files the submission created or changed, relative to the app root.
    pub touched: Vec<String>,
}

impl Outcome {
    /// `true` when every gate exited zero — the bar for reaching the judge.
    fn admitted(&self) -> bool {
        !self.gates.is_empty() && self.gates.values().all(|code| *code == 0)
    }
}

/// Load every task in `evals/tasks/`.
///
/// # Errors
/// When the tasks directory is missing or a `task.json` is malformed.
pub fn load_tasks(project_dir: &Path) -> Result<Vec<Task>> {
    let root = project_dir.join(TASKS_DIR);
    if !root.is_dir() {
        bail!("{} does not exist", root.display());
    }
    let mut tasks = Vec::new();
    for entry in fs::read_dir(&root)? {
        let dir = entry?.path();
        if !dir.is_dir() {
            continue;
        }
        let manifest = dir.join("task.json");
        if !manifest.is_file() {
            bail!("{} has no task.json", dir.display());
        }
        let meta: Value = serde_json::from_str(&fs::read_to_string(&manifest)?)?;
        let id = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        tasks.push(Task {
            title: meta["title"].as_str().unwrap_or_default().to_string(),
            principles: meta["principles"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|p| p.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
            app_flags: meta["app_flags"].as_array().map_or_else(
                || {
                    ["--db", "sqlite", "--bg", "async", "--assets", "serverside"]
                        .iter()
                        .map(|s| (*s).to_string())
                        .collect()
                },
                |a| {
                    a.iter()
                        .filter_map(|f| f.as_str().map(str::to_string))
                        .collect()
                },
            ),
            skill: string_list(&meta, "skill"),
            produce: string_list(&meta, "produce"),
            append: string_list(&meta, "append"),
            id,
            dir,
        });
    }
    tasks.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(tasks)
}

/// Read a JSON array of strings, defaulting to empty.
fn string_list(meta: &Value, key: &str) -> Vec<String> {
    meta[key]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Print the task corpus.
///
/// # Errors
/// When the corpus cannot be loaded.
pub fn list(project_dir: &Path) -> Result<()> {
    let tasks = load_tasks(project_dir)?;
    println!("{} eval task(s):\n", tasks.len());
    for task in &tasks {
        println!(
            "  {:<20} {:<12} {}",
            task.id,
            task.principles.join(","),
            task.title
        );
    }
    Ok(())
}

/// Gate every reference solution.
///
/// A reference that does not compile, lint, and test clean is not a reference —
/// and because every score is measured against it, a broken one corrupts the
/// whole series silently. This is the corpus's own test suite.
///
/// # Errors
/// When any reference fails its gates.
pub fn check_references(project_dir: &Path, work: &Path, only: Option<&str>) -> Result<()> {
    let tasks = load_tasks(project_dir)?;
    let shared_target = work.join("target");
    let mut failures = Vec::new();

    for task in &tasks {
        if only.is_some_and(|id| id != task.id) {
            continue;
        }
        if !task.reference().is_dir() {
            failures.push(format!("{}: no reference/ directory", task.id));
            continue;
        }
        println!("\n=== {} (reference) ===", task.id);
        let (app, _) = scaffold(project_dir, work, task, "reference")?;
        let mut touched = Vec::new();
        overlay(&task.wiring(), &task.wiring(), &app, &mut touched)?;
        for (rel, body) in task.appends()? {
            append_to(&app.join(&rel), &body)?;
        }
        let outcome = apply_and_gate(&app, &task.reference(), &shared_target)?;
        for (gate, code) in &outcome.gates {
            println!("  {gate}: {}", if *code == 0 { "pass" } else { "FAIL" });
        }
        if !outcome.admitted() {
            failures.push(format!(
                "{}: reference failed its own gates ({:?})",
                task.id, outcome.gates
            ));
        }
    }

    if failures.is_empty() {
        println!("\nall references pass their gates");
        return Ok(());
    }
    bail!("broken reference solutions:\n  {}", failures.join("\n  "));
}

/// Create (or reuse) a scaffolded Loco app for a task.
///
/// Apps are cached by their `loco new` flag signature and share one
/// `CARGO_TARGET_DIR`. Without that sharing every task pays a ~70s cold build
/// and the loop is unusable; with it, the second and later builds are seconds.
/// The pristine `loco new` scaffold for a task's flags, created on first use
/// and shared by every task with the same flag signature.
fn pristine_app(project_dir: &Path, work: &Path, task: &Task) -> Result<PathBuf> {
    let signature = task.app_flags.join("_").replace(['-', '/'], "");
    let pristine = work.join("apps").join(&signature);
    let app = pristine.join("app");

    if !app.is_dir() {
        fs::create_dir_all(&pristine)?;
        let mut args = vec!["new".to_string(), "-n".to_string(), "app".to_string()];
        args.extend(task.app_flags.iter().cloned());
        args.push("-a".to_string());
        duct::cmd("loco", &args)
            .dir(&pristine)
            .env("LOCO_DEV_MODE_PATH", project_dir)
            .run()
            .map_err(|e| eyre::eyre!("`loco new` failed — is the CLI installed? ({e})"))?;
    }
    Ok(app)
}

fn scaffold(
    project_dir: &Path,
    work: &Path,
    task: &Task,
    slot: &str,
) -> Result<(PathBuf, PathBuf)> {
    let app = pristine_app(project_dir, work, task)?;

    // Every run starts from the pristine scaffold, so one task cannot inherit
    // another's files.
    let run = work.join("runs").join(format!("{}-{slot}", task.id));
    if run.exists() {
        fs::remove_dir_all(&run)?;
    }
    copy_tree(&app, &run)?;
    Ok((run, app))
}

/// Overlay a solution onto an app and run the gates.
fn apply_and_gate(app: &Path, solution: &Path, target: &Path) -> Result<Outcome> {
    let before = fs::read_to_string(app.join("Cargo.toml")).unwrap_or_default();

    let mut touched = Vec::new();
    overlay(solution, solution, app, &mut touched)?;
    touched.sort();

    let after = fs::read_to_string(app.join("Cargo.toml")).unwrap_or_default();

    Ok(Outcome {
        gates: run_gates(app, target)?,
        added_deps: added_dependencies(&before, &after),
        touched,
    })
}

/// Compile, lint, and test an app, recording each gate's exit code.
fn run_gates(app: &Path, target: &Path) -> Result<BTreeMap<String, i32>> {
    let mut gates = BTreeMap::new();
    for (name, args) in [
        ("build", vec!["build"]),
        (
            "clippy",
            vec!["clippy", "--all-targets", "--", "-D", "warnings"],
        ),
        ("test", vec!["test"]),
    ] {
        let code = duct::cmd("cargo", &args)
            .dir(app)
            .env("CARGO_TERM_COLOR", "never")
            // One target dir across every task and arm. S2 measured the
            // difference: ~70s cold per app, versus ~8s once it is warm.
            .env("CARGO_TARGET_DIR", target)
            .unchecked()
            .stdout_capture()
            .stderr_capture()
            .run()?
            .status
            .code()
            .unwrap_or(-1);
        gates.insert(name.to_string(), code);
        // A submission that does not compile cannot lint or test; running them
        // anyway just reprints the same errors.
        if code != 0 {
            break;
        }
    }
    Ok(gates)
}

/// Crates present in `after`'s `[dependencies]` but not `before`'s.
fn added_dependencies(before: &str, after: &str) -> Vec<String> {
    let names = |manifest: &str| -> Vec<String> {
        manifest
            .lines()
            .skip_while(|l| l.trim() != "[dependencies]")
            .skip(1)
            .take_while(|l| !l.trim_start().starts_with('['))
            .filter_map(|l| l.split_once('=').map(|(name, _)| name.trim().to_string()))
            .filter(|n| !n.is_empty() && !n.starts_with('#'))
            .collect()
    };
    let existing = names(before);
    names(after)
        .into_iter()
        .filter(|n| !existing.contains(n))
        .collect()
}

/// Copy every file in `src` over `dst`, recording relative paths.
fn overlay(root: &Path, src: &Path, dst: &Path, touched: &mut Vec<String>) -> Result<()> {
    for entry in fs::read_dir(src)? {
        let path = entry?.path();
        let rel = path.strip_prefix(root)?;
        let out = dst.join(rel);
        if path.is_dir() {
            fs::create_dir_all(&out)?;
            overlay(root, &path, dst, touched)?;
        } else {
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&path, &out)?;
            touched.push(rel.display().to_string());
        }
    }
    Ok(())
}

/// Run the eval and report the KPI.
///
/// # Cost shape
///
/// Every `claude -p` invocation carries a fixed ~13k-token floor before it
/// reads a word of the prompt, so the number of *calls* dominates quota use,
/// not their size. This runs **one generation call per arm** with every case
/// batched into it, and **one judge call per arm** — four calls for the whole
/// suite, however many cases it holds. Per-case calls would multiply that
/// floor by the corpus size for no extra signal.
///
/// Generation is tool-free and one-shot. Letting the agent loose in a live app
/// measures its file-exploration stamina, which is not the claim under test;
/// the claim is that the material makes it write expert Loco.
///
/// Compilation is the objective half of the KPI and costs no quota at all —
/// `rustc` is the hallucination detector, and it is free.
///
/// # Errors
/// When scaffolding, the agent CLI, or the judge fails.
pub fn run(
    project_dir: &Path,
    work: &Path,
    only: Option<&str>,
    judge: bool,
    rejudge: bool,
    resume: bool,
) -> Result<()> {
    let all = load_tasks(project_dir)?;
    let tasks: Vec<&Task> = all
        .iter()
        .filter(|t| only.is_none_or(|id| id == t.id))
        .collect();
    if tasks.is_empty() {
        bail!("no matching tasks");
    }
    let shared_target = work.join("target");

    println!(
        "{} case(s), 2 arms: 2 generation call(s){}. Compilation is free.",
        tasks.len(),
        if judge {
            " + 4 judge call(s) (each arm scored in both orders)"
        } else {
            ""
        }
    );

    let mut spent = 0.0;
    let mut rows: Vec<Row> = Vec::new();

    for arm in [Arm::WithSkill, Arm::Bare] {
        println!("\n=== generating [{}] ===", arm.label());
        // Generation is the only billed step before judging, so a run that
        // died in the free part — compiling — must not pay for it twice.
        let cached = resume && answers_path(work, arm).is_file();
        if cached {
            println!("  reusing saved answers (not re-billed)");
        }
        let answers = if rejudge || cached {
            load_answers(work, arm)?
        } else {
            let (answers, cost) = generate(project_dir, work, &tasks, arm)?;
            spent += cost;
            save_answers(work, arm, &answers)?;
            answers
        };
        println!("  {} case(s) answered", answers.len());

        for task in &tasks {
            let files = answers.get(&task.id).cloned().unwrap_or_default();
            let (compiles, failure) = if files.is_empty() {
                (false, "no answer".to_string())
            } else {
                compile_case(project_dir, work, &shared_target, task, arm, &files)?
            };
            println!(
                "  {:<16} produced {} file(s), compiles: {}",
                task.id,
                files.len(),
                if compiles {
                    "yes".to_string()
                } else {
                    format!("NO  [{failure}]")
                }
            );
            rows.push(Row {
                id: task.id.clone(),
                arm,
                answered: !files.is_empty(),
                compiles,
                failure,
                files,
                score: None,
                spread: None,
            });
        }

        if judge {
            println!("=== judging [{}] ===", arm.label());
            // Both orders, always. One pass gives a number with no way to tell
            // whether it would survive being asked again.
            let mut passes: Vec<BTreeMap<String, f64>> = Vec::new();
            for order in [Order::SubmissionFirst, Order::ReferenceFirst] {
                let (scores, cost) = judge_arm(project_dir, &tasks, &rows, arm, order)?;
                spent += cost;
                println!("  {} cost: ${cost:.2}", order.label());
                passes.push(scores);
            }
            for row in rows.iter_mut().filter(|r| r.arm == arm) {
                let seen: Vec<f64> = passes
                    .iter()
                    .filter_map(|p| p.get(&row.id))
                    .copied()
                    .collect();
                row.score = average(&seen);
                row.spread = (seen.len() == 2).then(|| (seen[0] - seen[1]).abs());
            }
        }
    }

    report(&rows, spent);
    Ok(())
}

/// Where an arm's generated answers are cached between runs.
fn answers_path(work: &Path, arm: Arm) -> PathBuf {
    work.join("artifacts")
        .join(arm.label())
        .join("answers.json")
}

/// Persist an arm's answers so a later failure cannot waste the spend.
fn save_answers(work: &Path, arm: Arm, answers: &Answers) -> Result<()> {
    let path = answers_path(work, arm);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(answers)?)?;
    Ok(())
}

/// Reload answers from a previous run, for `--rejudge`.
fn load_answers(work: &Path, arm: Arm) -> Result<Answers> {
    let path = answers_path(work, arm);
    let body = fs::read_to_string(&path).map_err(|e| {
        eyre::eyre!(
            "no saved answers at {} ({e}); run without --rejudge",
            path.display()
        )
    })?;
    Ok(serde_json::from_str(&body)?)
}

/// One case under one arm.
struct Row {
    id: String,
    arm: Arm,
    /// Whether the generation call produced any file for this case at all.
    answered: bool,
    compiles: bool,
    /// Rustc error codes when `compiles` is false; empty otherwise.
    failure: String,
    files: BTreeMap<String, String>,
    /// Mean of the two order-swapped judge passes.
    score: Option<f64>,
    /// Absolute gap between those two passes — the error bar on `score`.
    spread: Option<f64>,
}

/// Which side of the judge prompt the submission is rendered on.
///
/// The same submission judged in both positions gives the run its error bar.
#[derive(Clone, Copy)]
enum Order {
    SubmissionFirst,
    ReferenceFirst,
}

impl Order {
    const fn label(self) -> &'static str {
        match self {
            Self::SubmissionFirst => "submission-first",
            Self::ReferenceFirst => "reference-first",
        }
    }
}

/// One arm of the A/B: whether the app carries the `loco` agent skill.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Arm {
    /// `loco new` output as shipped, skill included.
    WithSkill,
    /// Skill removed — the control, and the state every pre-existing app is in.
    Bare,
}

impl Arm {
    const fn label(self) -> &'static str {
        match self {
            Self::WithSkill => "with-skill",
            Self::Bare => "bare",
        }
    }
}

/// A batched answer: case id -> file path -> file contents.
type Answers = BTreeMap<String, BTreeMap<String, String>>;

/// Ask the model to answer every case in a single tool-free call.
///
/// The call is tool-free, so the agent cannot `cat` the app it is editing. The
/// prompts used to compensate with a hand-written "context you can rely on"
/// block per case — and that block was the eval's biggest methodology defect:
/// curating *which* facts matter is the expert judgement under test, and the
/// prose leaked framework answers (that `Authenticable` is the trait to
/// implement, that `find_by_pid` yields `ModelError::EntityNotFound`) to the
/// control arm for free. Both arms now get the app's real source verbatim
/// instead: exactly what one `cat` would show a tool-using agent, curated by
/// nobody.
fn generate(project_dir: &Path, work: &Path, tasks: &[&Task], arm: Arm) -> Result<(Answers, f64)> {
    let mut prompt = String::new();

    if arm == Arm::WithSkill {
        // The union of what each case declares it needs, deduplicated — the
        // same files `loco new` puts in the app.
        let mut wanted: Vec<&String> = tasks.iter().flat_map(|t| &t.skill).collect();
        wanted.sort();
        wanted.dedup();
        for rel in wanted {
            let body = fs::read_to_string(project_dir.join("skills/loco").join(rel))?;
            let _ = write!(prompt, "# Reference material: {rel}\n\n{body}\n\n---\n\n");
        }
    }

    prompt.push_str(
        "You are writing code for an existing Loco (loco-rs) application.\n\n\
         Answer EVERY task below. For each one, emit a header line `## CASE <id>`\n\
         followed by one fenced rust block per file you are asked to produce. Start\n\
         each block with a comment naming the file, exactly:\n\n\
         ```rust\n// file: src/path/to/file.rs\n...code...\n```\n\n\
         Write the complete file contents for a `// file:` block.\n\n\
         Some cases also ask you to EXTEND an existing file. For those you MUST\n\
         use `// append: <path>` (never `// file:`), containing ONLY the new code\n\
         to add. Rust permits several inherent `impl` blocks for one type, so an\n\
         extra `impl Model {{ ... }}` is valid and is what you should write.\n\
         Emitting the whole file instead will drop everything already in it and\n\
         break the build.\n\n\
         Module registration and route wiring are already handled. No prose\n\
         outside the blocks.\n\n---\n\n",
    );

    // Identical for both arms: the app source, verbatim, in place of a
    // hand-written summary of it.
    prompt.push_str(
        "# The app you are working in\n\n\
         These files already exist. They are shown in full so you do not have to\n\
         guess at their contents; read them as you would if you had opened them.\n\n",
    );
    let app = pristine_app(project_dir, work, tasks[0])?;
    for rel in APP_CONTEXT {
        let body = fs::read_to_string(app.join(rel))
            .map_err(|e| eyre::eyre!("reading app context file {rel}: {e}"))?;
        let _ = write!(prompt, "`{rel}`:\n\n```rust\n{body}\n```\n\n");
    }
    prompt.push_str("---\n\n");

    for task in tasks {
        let _ = write!(
            prompt,
            "## CASE {}\n\n{}\n\nProduce exactly these files: {}\n{}\n---\n\n",
            task.id,
            task.prompt()?,
            task.produce.join(", "),
            if task.append.is_empty() {
                String::new()
            } else {
                format!("Extend these existing files: {}\n", task.append.join(", "))
            }
        );
    }

    let (text, cost) = claude(&prompt, None)?;
    Ok((parse_cases(&text), cost))
}

/// Split a batched response into `case id -> {file path -> contents}`.
fn parse_cases(text: &str) -> Answers {
    let mut cases = BTreeMap::new();
    let mut current: Option<String> = None;
    let mut in_block = false;
    let mut file: Option<String> = None;
    let mut body = String::new();

    for line in text.lines() {
        if let Some(id) = line.trim().strip_prefix("## CASE ") {
            current = Some(id.trim().to_string());
            continue;
        }
        if line.trim_start().starts_with("```") {
            if in_block {
                if let (Some(id), Some(path)) = (current.clone(), file.take()) {
                    cases
                        .entry(id)
                        .or_insert_with(BTreeMap::new)
                        .insert(path, std::mem::take(&mut body));
                }
                body.clear();
            }
            in_block = !in_block;
            continue;
        }
        if in_block {
            if file.is_none() {
                // The `// file:` marker is the first line of each block.
                if let Some(path) = line.trim().strip_prefix("// file:") {
                    file = Some(path.trim().to_string());
                    continue;
                }
                if let Some(path) = line.trim().strip_prefix("// append:") {
                    file = Some(format!("{APPEND_PREFIX}{}", path.trim()));
                    continue;
                }
            }
            body.push_str(line);
            body.push('\n');
        }
    }
    cases
}

/// Build one case's answer inside a real app.
///
/// The reference overlay goes down first — it carries the module and route
/// wiring — and the generated files are written over the top. So the compile
/// tests the substance the agent actually wrote, not its ability to remember
/// `mod` declarations, which the prompt already grants it.
fn compile_case(
    project_dir: &Path,
    work: &Path,
    shared_target: &Path,
    task: &Task,
    arm: Arm,
    files: &BTreeMap<String, String>,
) -> Result<(bool, String)> {
    let (app, _) = scaffold(project_dir, work, task, arm.label())?;
    let mut touched = Vec::new();
    overlay(&task.wiring(), &task.wiring(), &app, &mut touched)?;

    for (rel, body) in files {
        // Never let a generated path escape the app.
        if rel.contains("..") {
            continue;
        }
        if let Some(target) = rel.strip_prefix(APPEND_PREFIX) {
            append_to(&app.join(target), body)?;
            continue;
        }
        let out = app.join(rel);
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(out, body)?;
    }

    // build + clippy only: the reference's request tests are wiring the agent
    // was never shown, so failing them would measure the wrong thing.
    let gates = run_gates(&app, shared_target)?;
    let ok = ["build", "clippy"]
        .iter()
        .all(|g| gates.get(*g).copied().unwrap_or(-1) == 0);

    // Clear before writing. A case that failed last run and passes this one
    // must not keep its old `BUILD_ERRORS.txt`, and a file the agent no longer
    // produces must not survive as evidence — stale artifacts get read back as
    // this run's result and turn a diagnosis into a fiction.
    let artifacts = work.join("artifacts").join(arm.label()).join(&task.id);
    let _ = fs::remove_dir_all(&artifacts);
    fs::create_dir_all(&artifacts)?;
    for (rel, body) in files {
        let name = rel.replace(['/', ':'], "_");
        fs::write(artifacts.join(name), body)?;
    }
    if !ok {
        let log = duct::cmd!("cargo", "build")
            .dir(&app)
            .env("CARGO_TERM_COLOR", "never")
            .env("CARGO_TARGET_DIR", shared_target)
            .unchecked()
            .stderr_to_stdout()
            .stdout_capture()
            .read()
            .unwrap_or_default();
        let class = failure_class(&log);
        fs::write(artifacts.join("BUILD_ERRORS.txt"), log)?;
        return Ok((false, class));
    }
    Ok((true, String::new()))
}

/// The distinct rustc error codes in a build log, worst-first as emitted.
///
/// Runs 6 and 7 both scored 86% on compile rate while the two arms were failing
/// the *same case for different reasons* — one a missing trait import, the other
/// an orphan-rule violation. A pass/fail bit cannot show that and the mean is
/// saturated, so the failure class is the number that still carries signal.
fn failure_class(log: &str) -> String {
    let mut codes: Vec<String> = Vec::new();
    for line in log.lines() {
        let Some(rest) = line.trim_start().strip_prefix("error[") else {
            continue;
        };
        if let Some(code) = rest.split(']').next() {
            let code = code.to_string();
            if !codes.contains(&code) {
                codes.push(code);
            }
        }
    }
    if codes.is_empty() {
        // No coded error: a lint denial, a missing file, or a link failure.
        return log
            .lines()
            .find(|l| l.trim_start().starts_with("error: "))
            .map_or_else(
                || "unknown".to_string(),
                |l| l.trim().chars().take(60).collect(),
            );
    }
    codes.join(",")
}

/// Append a snippet to an existing file, creating it when absent.
fn append_to(path: &Path, body: &str) -> Result<()> {
    let mut existing = fs::read_to_string(path).unwrap_or_default();
    existing.push('\n');
    existing.push_str(body);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, existing)?;
    Ok(())
}

/// Every file under `dir`, relative to `root`.
fn collect_files(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_files(root, &path, out)?;
        } else {
            out.push(path.strip_prefix(root)?.to_path_buf());
        }
    }
    Ok(())
}

/// Score one arm's answers against the references, in a single judge call.
///
/// `order` decides which side is rendered first. Run both ways and the gap
/// between the results is the error bar; run one way, as runs 1-9 did, and
/// there is no way to tell a real delta from the judge changing its mind.
fn judge_arm(
    project_dir: &Path,
    tasks: &[&Task],
    rows: &[Row],
    arm: Arm,
    order: Order,
) -> Result<(BTreeMap<String, f64>, f64)> {
    let doctrine = fs::read_to_string(project_dir.join(DOCTRINE))?;
    let rubric = fs::read_to_string(project_dir.join(RUBRIC))?;

    let mut body = String::new();
    for task in tasks {
        let Some(row) = rows.iter().find(|r| r.arm == arm && r.id == task.id) else {
            continue;
        };

        let mut submission = String::from("### Submission\n\n");
        if row.files.is_empty() {
            submission.push_str("(no answer produced)\n\n");
        } else {
            for (rel, code) in &row.files {
                let _ = write!(submission, "`{rel}`\n\n```rust\n{code}\n```\n\n");
            }
        }

        let mut reference = String::from("### Reference\n\n");
        for rel in &task.produce {
            let code = fs::read_to_string(task.reference().join(rel)).unwrap_or_default();
            let _ = write!(reference, "`{rel}`\n\n```rust\n{code}\n```\n\n");
        }
        for (rel, code) in task.appends()? {
            let _ = write!(reference, "appended to `{rel}`\n\n```rust\n{code}\n```\n\n");
        }

        let _ = write!(body, "## CASE {}\n\n", task.id);
        match order {
            Order::SubmissionFirst => {
                body.push_str(&submission);
                body.push_str(&reference);
            }
            Order::ReferenceFirst => {
                body.push_str(&reference);
                body.push_str(&submission);
            }
        }
        body.push_str("---\n\n");
    }

    let prompt = format!(
        "{doctrine}\n\n---\n\n{rubric}\n\n---\n\n{body}\n---\n\n\
         For EACH case above, score the Submission against the Reference on the \
         five criteria. The Reference is a Loco maintainer's answer; it is the \
         standard, not a competitor. Respond with ONLY this JSON, no prose:\n\
         {{\"cases\":[{{\"id\":\"<case id>\",\"c1\":n,\"c2\":n,\"c3\":n,\"c4\":n,\"c5\":n,\
         \"defects\":[\"<worst first, each citing a symbol or line>\"]}}]}}"
    );

    let (text, cost) = claude(&prompt, Some(JUDGE_MODEL))?;
    let raw_dir = project_dir.join("target/eval/artifacts").join(arm.label());
    fs::create_dir_all(&raw_dir)?;
    // Both passes rewrite their own file every run, so neither goes stale. The
    // single-pass era's unsuffixed file would, though — and sitting beside two
    // current ones it reads as a third result rather than as a leftover.
    let _ = fs::remove_file(raw_dir.join("JUDGE_RAW.json"));
    fs::write(
        raw_dir.join(format!("JUDGE_RAW.{}.json", order.label())),
        &text,
    )?;

    // A single stray brace used to discard an entire run's scores — including
    // the generation spend that produced them. Try the whole document, then
    // fall back to salvaging each case object on its own.
    let cases: Vec<Value> = serde_json::from_str::<Value>(extract_json(&text))
        .ok()
        .and_then(|v| v["cases"].as_array().cloned())
        .unwrap_or_else(|| salvage_objects(&text));
    if cases.is_empty() {
        bail!("judge returned nothing parseable: {text}");
    }

    let mut scores = BTreeMap::new();
    for case in &cases {
        let Some(id) = case["id"].as_str() else {
            continue;
        };
        let criteria = ["c1", "c2", "c3", "c4", "c5"];
        let values: Vec<f64> = criteria.iter().filter_map(|c| case[*c].as_f64()).collect();
        if let Some(mean) = average(&values) {
            scores.insert(id.to_string(), mean);
        }
        // Tagged by pass: which defects the judge names in only one of the two
        // orders is as telling as the score gap between them.
        if let Some(defects) = case["defects"].as_array() {
            for defect in defects.iter().filter_map(Value::as_str) {
                println!("    [{id}] ({}) {defect}", order.label());
            }
        }
    }
    Ok((scores, cost))
}

/// Pull out every top-level `{...}` that carries an `"id"`, parsing each alone.
///
/// Models occasionally emit an extra brace or a trailing comma in a long JSON
/// document. Scoring six good cases is better than discarding all seven.
fn salvage_objects(text: &str) -> Vec<Value> {
    let bytes: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != '{' {
            i += 1;
            continue;
        }
        let (mut depth, mut j, mut in_string, mut escaped) = (0_i32, i, false, false);
        while j < bytes.len() {
            let c = bytes[j];
            if in_string {
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == '"' {
                    in_string = false;
                }
            } else if c == '"' {
                in_string = true;
            } else if c == '{' {
                depth += 1;
            } else if c == '}' {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            j += 1;
        }
        if depth != 0 || j >= bytes.len() {
            break;
        }
        let candidate: String = bytes[i..=j].iter().collect();
        if let (true, Ok(value)) = (
            candidate.contains("\"id\""),
            serde_json::from_str::<Value>(&candidate),
        ) {
            out.push(value);
            i = j + 1;
            continue;
        }
        i += 1;
    }
    out
}

/// One tool-free `claude -p` call, returning its text and what it cost.
fn claude(prompt: &str, model: Option<&str>) -> Result<(String, f64)> {
    let mut args = vec![
        "-p".to_string(),
        prompt.to_string(),
        "--tools".to_string(),
        String::new(),
        "--output-format".to_string(),
        "json".to_string(),
    ];
    if let Some(model) = model {
        args.push("--model".to_string());
        args.push(model.to_string());
    }

    // One retry: a transient CLI failure previously destroyed a run's judging
    // along with the generation spend that had already been paid for.
    let mut last = String::new();
    for attempt in 0..2 {
        let out = duct::cmd("claude", &args)
            .stdout_capture()
            .stderr_capture()
            .unchecked()
            .run()?;
        if out.status.success() {
            last = String::from_utf8_lossy(&out.stdout).into_owned();
            break;
        }
        let err = String::from_utf8_lossy(&out.stderr);
        // Truncate: duct echoes the entire prompt back on failure.
        let reason: String = err.chars().take(400).collect();
        if attempt == 1 {
            bail!("claude CLI failed twice: {reason}");
        }
        eprintln!("  claude call failed, retrying once: {reason}");
    }
    let envelope: Value = serde_json::from_str(&last)
        .map_err(|e| eyre::eyre!("claude CLI returned non-JSON: {e}"))?;
    Ok((
        envelope
            .get("result")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        envelope
            .get("total_cost_usd")
            .and_then(Value::as_f64)
            .unwrap_or(0.0),
    ))
}

/// Pull the first JSON object out of a model response that may be fenced.
fn extract_json(raw: &str) -> &str {
    let start = raw.find('{').unwrap_or(0);
    let end = raw.rfind('}').map_or(raw.len(), |i| i + 1);
    raw.get(start..end).unwrap_or(raw)
}

/// Print the per-case detail and the headline KPI.
/// Arithmetic mean, or `None` for an empty slice.
///
/// The `usize -> f64` cast clippy warns about is exact by construction here:
/// these are counts of eval cases and judged criteria — tens of items, not the
/// 2^53 where an `f64` starts losing integers.
#[allow(clippy::cast_precision_loss)]
fn average(xs: &[f64]) -> Option<f64> {
    (!xs.is_empty()).then(|| xs.iter().sum::<f64>() / xs.len() as f64)
}

/// A count as a float, for ratios. Same reasoning as [`average`].
#[allow(clippy::cast_precision_loss)]
const fn count_as_f64(n: usize) -> f64 {
    n as f64
}

fn report(rows: &[Row], spent: f64) {
    println!("\n{:-<104}", "");
    println!(
        "{:<18} {:<11} {:>9} {:>10} {:>9} {:>8}  failure",
        "case", "arm", "answered", "compiles", "idiomatic", "spread"
    );
    println!("{:-<104}", "");
    for row in rows {
        println!(
            "{:<18} {:<11} {:>9} {:>10} {:>9} {:>8}  {}",
            row.id,
            row.arm.label(),
            yes_no(row.answered),
            yes_no(row.compiles),
            row.score
                .map_or_else(|| "-".to_string(), |s| format!("{s:.2}/5")),
            row.spread
                .map_or_else(|| "-".to_string(), |s| format!("{s:.2}")),
            row.failure,
        );
    }
    println!("{:-<104}", "");

    // A case both arms fail is not necessarily the same failure in both. Runs 6
    // and 7 hid exactly that behind an identical compile rate, so name it here
    // rather than leaving it to whoever opens the artifacts.
    let mut split: Vec<String> = Vec::new();
    for row in rows
        .iter()
        .filter(|r| r.arm == Arm::WithSkill && !r.compiles)
    {
        if let Some(other) = rows
            .iter()
            .find(|r| r.arm == Arm::Bare && r.id == row.id && !r.compiles)
            .filter(|other| other.failure != row.failure)
        {
            split.push(format!(
                "  {:<18} with-skill {} vs bare {}",
                row.id, row.failure, other.failure
            ));
        }
    }
    if !split.is_empty() {
        println!("\nfailed in both arms, but not for the same reason:");
        for line in split {
            println!("{line}");
        }
    }

    println!("\nKPI — does the material make an agent write expert Loco?\n");
    println!(
        "{:<12} {:>12} {:>16} {:>12}",
        "arm", "compile rate", "mean idiomatic", "cases"
    );
    for arm in [Arm::WithSkill, Arm::Bare] {
        let arm_rows: Vec<&Row> = rows.iter().filter(|r| r.arm == arm).collect();
        if arm_rows.is_empty() {
            continue;
        }
        let compiled = arm_rows.iter().filter(|r| r.compiles).count();
        let scored: Vec<f64> = arm_rows.iter().filter_map(|r| r.score).collect();
        println!(
            "{:<12} {:>11.0}% {:>15} {:>12}",
            arm.label(),
            100.0 * count_as_f64(compiled) / count_as_f64(arm_rows.len()),
            average(&scored).map_or_else(|| "-".to_string(), |m| format!("{m:.2}/5")),
            arm_rows.len(),
        );
    }

    // The delta is the claim. Report it explicitly rather than making the
    // reader subtract two rows and decide what counts as an improvement.
    let delta = |pick: &dyn Fn(&Row) -> Option<f64>| -> Option<f64> {
        let arm_mean = |arm: Arm| {
            let vals: Vec<f64> = rows
                .iter()
                .filter(|r| r.arm == arm)
                .filter_map(pick)
                .collect();
            average(&vals)
        };
        Some(arm_mean(Arm::WithSkill)? - arm_mean(Arm::Bare)?)
    };
    if let Some(d) = delta(&|r| Some(if r.compiles { 1.0 } else { 0.0 })) {
        println!("\ncompile-rate delta : {:+.0} points", d * 100.0);
    }
    if let Some(d) = delta(&|r| r.score) {
        println!("idiomatic delta    : {d:+.2} / 5");

        // The delta is only a result if it clears the judge's own inconsistency
        // on identical code. Print the comparison rather than the ingredients:
        // nine runs' worth of ±0.11 deltas were read as signal without it.
        let spreads: Vec<f64> = rows.iter().filter_map(|r| r.spread).collect();
        if let Some(mean) = average(&spreads) {
            let worst = spreads.iter().copied().fold(0.0_f64, f64::max);
            println!(
                "judge spread       : {mean:.2} mean, {worst:.2} worst (same code, both orders)"
            );
            if d.abs() <= mean {
                println!(
                    "  -> |delta| {:.2} does not clear the {mean:.2} spread: NOT a result.",
                    d.abs()
                );
            } else {
                println!("  -> |delta| {:.2} clears the {mean:.2} spread.", d.abs());
            }
        }
    }
    println!("\ntotal billed this run: ${spent:.2}");
}

fn yes_no(value: bool) -> String {
    if value { "yes" } else { "NO" }.to_string()
}

/// Recursively copy a directory tree.
fn copy_tree(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let path = entry?.path();
        let name = path.file_name().unwrap_or_default().to_owned();
        // Build output is regenerated into the shared target dir; copying it
        // per run costs gigabytes and buys nothing.
        if name == "target" {
            continue;
        }
        let out = dst.join(&name);
        if path.is_dir() {
            copy_tree(&path, &out)?;
        } else {
            fs::copy(&path, &out)?;
        }
    }
    Ok(())
}
