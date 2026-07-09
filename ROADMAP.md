# Roadmap
This roadmap is a working document. It will change as Roco evolves.

## Near term goals
- [ ] Complete migration from Loco to Roco
  - Rename code and documentation from Loco to Roco
  - Add fork attribution
  - Update repository links, badges, package metadata, and install instructions.
  - Document breaking changes before release.
- [ ] Prepare Cargo publishing
  - Decide package names such as `roco-rs`, `roco-cli`, and `roco-gen`.
  - Check crate name availability.
  - Set up release notes and changelog process.

## Short term goals
- [ ] Dependency updates
  - Track SeaORM major updates
  - Tera 2
- [ ] Verify CI and release process for Roco
  - Ensure formatting, clippy, tests and docs pass under Roco repository
  - Update release automation, badges, package ownership, and changelog flow.
  - Publish regular releases with migration notes.

## Medium term goals
- [ ] Improve authentication 
  - Magic links
  - OAuth/OIDC login.
  - Passkeys/WebAuthn.
- [ ] Authorization layer
  - Policy-based authorization layer similar to Pundit
- [ ] Build a first-class Websocket layer
  - Official Channel based adapter like Action Cable
  - Database backed channels similar to Solid Cable
- [ ] Add database-backed queues and jobs adapters 
  - Durable job processing inspired by Solid Queue and Sidekiq.
  - Support both Redis and Database backed Adapters.
- [ ] Compile time improvements
  - Measure current bottlenecks.
  - Considering splitting optional subsystems into separate crates for compilation time.

## Long term goals
- [ ] Improvement of Deployment
  - Explore a Kamal like CLI for deploying to VPS
- [ ] Add support for encrypted secrets
  - Application secrets with environment support inspired by Rails credentials
- [ ] Add Python integration
  - Explore optional python integration for data science and AI projects
- [ ] Add support for Inertia.js to simplify frontend development with JavaScript frameworks
  - Explore an Inertia.js adapter for Roco