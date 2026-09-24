//! Coverage for the agent material every generated app ships.
//!
//! `setup.rhai` copies `.claude/` and `AGENTS.md` unconditionally, and nothing
//! asserted it. Drop that one line and every new app silently loses the skill
//! that makes a coding agent fluent in Loco — no test fails, no gate goes red,
//! and the loss is invisible until someone notices an agent guessing again.
//!
//! `cargo xtask agent-skill --check` guards the *contents* against drift. These
//! guard that the wizard actually emits them, for every choice a user can make.

use loco::{settings, wizard::AssetsOption};
use rstest::rstest;

use super::*;
use crate::assertion;

/// Every file under `base_template/.claude/skills/loco/`. Listed rather than
/// globbed: a new file has to be added here deliberately, and a deleted one has
/// to fail.
const SKILL_FILES: [&str; 17] = [
    "SKILL.md",
    "doctrine.md",
    "workflow.md",
    "errors.md",
    "api-index.md",
    "sea-orm-index.md",
    "starter-app.md",
    "recipes/auth.md",
    "recipes/background-job.md",
    "recipes/cache.md",
    "recipes/config.md",
    "recipes/endpoint.md",
    "recipes/mailer.md",
    "recipes/middleware.md",
    "recipes/model-and-migration.md",
    "recipes/task-and-schedule.md",
    "recipes/testing.md",
];

fn run_generator(asset: AssetsOption) -> TestGenerator {
    let settings = settings::Settings {
        asset: asset.into(),
        ..Default::default()
    };

    TestGenerator::generate(settings)
}

#[rstest]
fn every_app_ships_the_whole_loco_skill(
    #[values(AssetsOption::None, AssetsOption::Serverside, AssetsOption::Clientside)]
    asset: AssetsOption,
) {
    let generator = run_generator(asset);

    for file in SKILL_FILES {
        let path = generator.path(&format!(".claude/skills/loco/{file}"));
        assert!(
            path.exists(),
            "{file} is missing from a generated app — an agent working in it \
             loses that material entirely"
        );
        assert!(
            !std::fs::read_to_string(&path)
                .expect("read skill file")
                .trim()
                .is_empty(),
            "{file} ships empty"
        );
    }
}

#[rstest]
fn every_app_ships_agents_md(
    #[values(AssetsOption::None, AssetsOption::Serverside, AssetsOption::Clientside)]
    asset: AssetsOption,
) {
    let generator = run_generator(asset);
    let content = assertion::string::load(generator.path("AGENTS.md"));

    // The entry point has to point at the skill, or the skill is only found by
    // an agent that already went looking for it.
    assertion::string::assert_contains(&content, ".claude/skills/loco");
}
