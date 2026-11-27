// sea-orm writes `pub enum Relation {}` for tables with no relations, and
// `cargo loco db entities` rewrites these files, so the allow lives here.
// `unknown_lints` keeps toolchains older than the `empty_enums` lint quiet.
#[allow(unknown_lints, clippy::empty_enums)]
pub mod _entities;
{%- if settings.auth %}
pub mod users;
{%- endif %}