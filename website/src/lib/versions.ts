// Inlined by Vite at build time, resolved relative to THIS file. Reading the
// manifest at runtime instead looked fine in vitest and then failed the build:
// Astro bundles this module into a server chunk, so `import.meta.url` pointed
// at the bundle and the path collapsed to `website/Cargo.toml`. `?raw` is
// resolved while the module graph is still source-shaped, and leaves no file
// IO in the output at all.
import cargoToml from '../../../Cargo.toml?raw';

const CARGO_TOML = 'the workspace Cargo.toml';

/**
 * Reads `version` from the `[package]` section of a `Cargo.toml`.
 *
 * Scoped to that section deliberately: `version = "..."` also appears inside
 * every dependency's inline table, and `[workspace.package]` above it carries
 * its own keys. Anchoring on the section heading is what makes a bare
 * line-start match safe.
 *
 * @param manifest contents of a Cargo.toml
 * @returns e.g. `'1.1.0'`
 */
export function packageVersion(manifest: string): string {
  const section = manifest.split(/^\[package\]$/m)[1];
  if (section === undefined) {
    throw new Error(`${CARGO_TOML} has no [package] section`);
  }
  // Stop at the next section heading so a later `[dependencies]` entry cannot
  // be mistaken for the package's own version.
  const body = section.split(/^\[/m)[0];
  const match = body.match(/^version = "([^"]+)"/m);
  if (!match) {
    throw new Error(`${CARGO_TOML} [package] section declares no version`);
  }
  return match[1];
}

/**
 * The released Loco version, read from the workspace `Cargo.toml` at build
 * time.
 *
 * It used to be a hand-maintained list, and it drifted: the header still read
 * `v1.0` after 1.1.0 had shipped. A version string on the docs site that
 * nobody can trust is worse than no version string, and the only copy that
 * cannot go stale is the one derived from the manifest being released.
 */
export const locoVersion: string = packageVersion(cargoToml);

/** The GitHub release notes for {@link locoVersion}. */
export const locoReleaseUrl = `https://github.com/loco-rs/loco/releases/tag/v${locoVersion}`;
