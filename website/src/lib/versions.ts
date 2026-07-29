export interface DocVersion {
  /** Badge/menu label, e.g. `"v1.0"`. */
  label: string;
  /** Root docs path (or absolute URL) this version is served from. */
  href: string;
  /** Marks the version currently being served by this build. Exactly one entry should set this. */
  current?: boolean;
}

// Every published docs version, newest first. Loco docs are single-version
// today — this array has one entry — but `VersionBadge.astro` always
// renders it as a dropdown (see that component), so it is already wired for
// more without any template changes.
//
// To publish a new version later:
//   1. Preserve the outgoing version's content somewhere it can still be
//      served from its own `href` below — e.g. a versioned subpath such as
//      `/v1.0/docs/...` (a copy of today's `src/content/docs/docs/` tree),
//      or adopt the `starlight-versions` plugin to manage those snapshots
//      for you. Either way, don't touch this file until that content exists
//      and resolves at the URL you're about to add.
//   2. Add the new version as `current: true` and demote the previous
//      `current` entry to a plain historical entry pointing at its
//      preserved path, e.g.:
//        export const docVersions: DocVersion[] = [
//          { label: 'v2.0', href: '/docs/', current: true },
//          { label: 'v1.0', href: '/v1.0/docs/' },
//        ];
//   3. That's the whole change — `VersionBadge.astro` maps over this
//      array unmodified.
export const docVersions: DocVersion[] = [{ label: 'v1.0', href: '/docs/', current: true }];
