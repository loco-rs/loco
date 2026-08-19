import { describe, expect, it } from 'vitest';
import { locoReleaseUrl, locoVersion, packageVersion } from './versions';

describe('packageVersion', () => {
  it('reads the version out of [package], not out of a dependency', () => {
    const manifest = `[workspace]
members = ["xtask", "loco-gen"]

[workspace.package]
rust-version = "1.94"

[package]
name = "loco-rs"
version = "1.1.0"

[dependencies]
tera = { version = "2.1", features = ["glob_fs"] }
`;
    expect(packageVersion(manifest)).toBe('1.1.0');
  });

  it('is not fooled by a [dependencies] table that starts with a bare version key', () => {
    const manifest = `[package]
name = "loco-rs"
version = "1.1.0"

[dependencies.tera]
version = "2.1"
`;
    expect(packageVersion(manifest)).toBe('1.1.0');
  });

  it('throws rather than rendering a wrong version', () => {
    expect(() => packageVersion('[dependencies]\nversion = "9.9.9"\n')).toThrow(/\[package\]/);
    expect(() => packageVersion('[package]\nname = "loco-rs"\n')).toThrow(/no version/);
  });
});

describe('the version the site actually ships', () => {
  it('is a real semver read from the workspace manifest', () => {
    expect(locoVersion).toMatch(/^\d+\.\d+\.\d+/);
  });

  it('links at the matching release tag', () => {
    expect(locoReleaseUrl).toBe(`https://github.com/loco-rs/loco/releases/tag/v${locoVersion}`);
  });
});
