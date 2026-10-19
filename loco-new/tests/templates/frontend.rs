//! Coverage for the clientside SPA the wizard emits.
//!
//! The rest of this directory tests the *config* the asset choice produces;
//! nothing tested the emitted `frontend/` tree itself. Four defects survived a
//! green wizard matrix because of that gap — no sign-up page in an app whose
//! headline feature is built-in auth, a dev proxy hardcoded to a port the
//! backend lets you override, and a home page that never links anywhere.
//!
//! These are property assertions, not snapshots, on purpose. A snapshot freezes
//! whatever the template emitted the day it was written, so it cannot fail for a
//! defect that predates it. Every test here states something that must be true
//! of *any* future version of the template.

use loco::{settings, wizard::AssetsOption};
use rstest::rstest;

use super::*;
use crate::assertion;

fn run_generator(asset: AssetsOption) -> TestGenerator {
    let settings = settings::Settings {
        asset: asset.into(),
        ..Default::default()
    };

    TestGenerator::generate(settings)
}

#[test]
fn clientside_ships_a_signup_page() {
    let generator = run_generator(AssetsOption::Clientside);
    let content = assertion::string::load(generator.path("frontend/src/auth/Register.tsx"));

    assertion::string::assert_contains(&content, "/api/auth/register");
    for field in ["name", "email", "password"] {
        assertion::string::assert_contains(&content, field);
    }
}

#[test]
fn login_offers_signup_rather_than_instructions_to_curl() {
    let generator = run_generator(AssetsOption::Clientside);
    let content = assertion::string::load(generator.path("frontend/src/auth/Login.tsx"));

    // The page used to tell the reader to `POST /api/auth/register` by hand.
    // Anything that reads as an instruction to leave the app is the defect.
    assertion::string::assert_not_contains(&content, "curl");
    assertion::string::assert_not_contains(&content, "<code>POST /api/auth/register</code>");
    assertion::string::assert_contains(&content, "/register");
}

#[test]
fn the_signup_page_is_reachable() {
    let generator = run_generator(AssetsOption::Clientside);
    let content = assertion::string::load(generator.path("frontend/src/routes.tsx"));

    assertion::string::assert_contains(&content, "Register");
    assertion::string::assert_contains(&content, "'register'");
}

#[test]
fn the_dev_proxy_follows_the_port_the_backend_actually_uses() {
    let generator = run_generator(AssetsOption::Clientside);
    let content = assertion::string::load(generator.path("frontend/vite.config.ts"));

    // `config/development.yaml` renders `port: get_env(name="PORT", ...)`, so
    // the backend port is overridable. A literal here silently points the proxy
    // at 5150 the moment anyone overrides it.
    assertion::string::assert_not_contains(&content, "http://localhost:5150");
    assertion::string::assert_contains(&content, "PORT");
}

#[test]
fn the_home_page_can_link_to_scaffolded_resources() {
    let generator = run_generator(AssetsOption::Clientside);
    let content = assertion::string::load(generator.path("frontend/src/pages/Home.tsx"));

    // The scaffold injects nav entries here. Without the anchor the generated
    // app is a dead end: you scaffold a resource and nothing links to it.
    assertion::string::assert_contains(&content, "scaffold:nav");
}

#[rstest]
fn no_frontend_tree_without_the_clientside_choice(
    #[values(AssetsOption::None, AssetsOption::Serverside)] asset: AssetsOption,
) {
    let generator = run_generator(asset);

    for path in [
        "frontend/vite.config.ts",
        "frontend/src/routes.tsx",
        "frontend/src/auth/Register.tsx",
    ] {
        assert!(
            !generator.path(path).exists(),
            "{path} should not be generated for a non-clientside app"
        );
    }
}

#[test]
fn the_frontend_build_typechecks_before_it_bundles() {
    let generator = run_generator(AssetsOption::Clientside);
    let content = assertion::string::load(generator.path("frontend/package.json"));

    // `vite build` transpiles with esbuild, which strips types without checking
    // them — a type error bundles cleanly and ships. CI runs this same script,
    // so without `tsc` in front of it nothing in the project typechecks the SPA
    // at all. Verified by hand: a deliberate `42 as string` fails the build at
    // `tsc` and never reaches vite.
    assertion::string::assert_contains(&content, "tsc -b && vite build");
}
