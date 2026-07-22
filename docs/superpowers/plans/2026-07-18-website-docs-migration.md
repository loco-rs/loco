# Loco Website — Docs Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate the 64 Zola Diátaxis docs into the Astro/Starlight site, themed to the warm Loco identity, with working sidebar/search/links and `llms.txt`.

**Architecture:** The Astro `website/` project (from the Foundation plan) already wires Starlight at `/docs`. This plan (a) themes Starlight with the existing design tokens, (b) converts the 64 Zola `.md` files (TOML frontmatter → Starlight YAML) into `website/src/content/docs/docs/<section>/`, (c) rewrites Zola `@/`-style internal links to Starlight URLs, and (d) restores `llms.txt`. Files stay plain `.md` (not `.mdx`) so Loco's literal `{{ get_env(...) }}` config syntax survives untouched.

**Tech Stack:** Astro 5 + `@astrojs/starlight` 0.30, Node/TS conversion scripts, `@toml-tools`/`smol-toml` for TOML parsing, Vitest, pnpm.

## Global Constraints

- **Source is read-only:** the Zola docs in `docs-site/content/docs/` are the content source — do NOT modify `docs-site/`. Converted output goes to `website/src/content/docs/docs/<section>/` (note the nested `docs/docs/` — Starlight's collection root is `src/content/docs/`, and the extra `docs/` sub-dir yields `/docs/*` URLs, matching the old site; established in the Foundation plan).
- **Files stay `.md`, never `.mdx`.** Loco config examples contain literal `{{ get_env(name='JWT_SECRET') }}` Tera syntax inside code fences and inline code; plain Starlight markdown renders these verbatim. `.mdx` would try to evaluate `{…}` and break them. (Verified: zero `{{ }}` occurrences require execution — all are literal config syntax.)
- **Frontmatter mapping (exact):** Zola TOML `title` → `title`; `description` → `description`; `weight` (integer, present on all 64) → `sidebar: { order: <weight> }`; **drop** `sort_by`, `template`, `insert_anchor_links`, and any other Zola-only keys. A section's `_index.md` becomes that section directory's `index.md`.
- **Diátaxis section labels (sidebar):** `tutorials`→"Tutorials", `how-to`→"How-to guides", `reference`→"Reference", `explanation`→"Explanation", `extras`→"Extras", `resources`→"Resources". Section file counts: tutorials 4, how-to 35, reference 11, explanation 8, extras 2, resources 3.
- **Internal-link rewrite:** Zola absolute links `@/docs/<path>.md` (optionally `#anchor`) → Starlight `/docs/<path>` (drop `.md`, keep `#anchor`). No `@/…` link may remain after conversion.
- **Theme Starlight with the existing design tokens** (`website/src/styles/tokens.css`, light + dark) so docs match the homepage's warm-postcard identity — override Starlight's `--sl-color-*` custom properties; warm-dark code blocks; `#FC3820`/`#FF5A3D` accents. **Pixel reference:** `docs/superpowers/specs/2026-07-17-docs-starlight-reference.html` (committed) shows the intended docs look; the spec's "Docs (Starlight, themed)" section is the binding description.
- **Preserve `llms.txt` / `llms-full.txt`** (the old site shipped these AI-consumable docs bundles).
- **URL parity:** old docs URLs were `/docs/<section>/<slug>/`. Ensure the Starlight build serves the same paths (configure trailing slash) or add redirects; produce a before/after URL map and confirm no live doc URL 404s.
- English only.

---

## File Structure

```
website/
  astro.config.mjs                      # MODIFY: real Starlight sidebar (Diátaxis groups) + trailing-slash + llms-txt plugin
  src/styles/starlight.css              # CREATE: map tokens → Starlight --sl-color-* (light+dark), warm-dark code
  scripts/
    convert-frontmatter.mjs             # CREATE: Zola TOML → Starlight YAML, one file
    migrate-docs.mjs                    # CREATE: walk docs-site/content/docs, convert + write to content/docs/docs/**
    rewrite-links.mjs                   # CREATE: @/docs/*.md(#a) → /docs/*(#a) across converted files
    scripts.test.mjs                    # CREATE: Vitest for convert-frontmatter + rewrite-links (pure string transforms)
  src/content/docs/docs/                # CREATE (generated): 64 migrated .md across the six sections
    tutorials/ how-to/ reference/ explanation/ extras/ resources/
  public/llms.txt, public/llms-full.txt # CREATE: generated AI-docs bundles (or via plugin output)
```

---

### Task 1: Theme Starlight to the Loco identity + sidebar config

**Files:**
- Create: `website/src/styles/starlight.css`
- Modify: `website/astro.config.mjs` (add `customCss`, real `sidebar`, `trailingSlash`)

**Interfaces:**
- Produces: a themed Starlight whose `/docs` pages use the warm tokens; a sidebar with the six Diátaxis groups. Consumed visually by all later tasks.

- [ ] **Step 1: Write `starlight.css`** mapping our tokens onto Starlight's variables, for BOTH themes. Port the accent/paper/ink/code choices from the reference (`docs/superpowers/specs/2026-07-17-docs-starlight-reference.html`). At minimum set: `--sl-color-accent`, `--sl-color-accent-low/high`, `--sl-color-bg`, `--sl-color-bg-nav`, `--sl-color-bg-sidebar`, `--sl-color-text`, `--sl-color-text-accent`, `--sl-color-hairline`, and the code-block background to the warm-dark palette. Use `:root[data-theme='dark']` / `[data-theme='light']` to align with the existing toggle (Starlight uses `data-theme` too — verify the toggle from the Foundation plan and Starlight's theme state don't conflict; reconcile to one mechanism and note it).

```css
/* website/src/styles/starlight.css — import tokens, then override Starlight vars */
@import './tokens.css';
:root {
  --sl-color-accent: var(--red);
  --sl-color-accent-high: var(--red-ink);
  --sl-color-text-accent: var(--red-ink);
  --sl-color-bg: var(--paper);
  --sl-color-bg-nav: var(--paper);
  --sl-color-bg-sidebar: var(--paper-2);
  --sl-color-hairline: var(--line);
  --sl-color-text: var(--ink);
  --sl-color-gray-1: var(--ink); --sl-color-gray-2: var(--ink-2); --sl-color-gray-3: var(--ink-3);
}
/* warm-dark code blocks in both themes */
.expressive-code, :root { --sl-color-bg-inline-code: var(--paper-2); }
```
(Adjust variable names to the installed Starlight version — verify against `node_modules/@astrojs/starlight` if a name doesn't take effect.)

- [ ] **Step 2: Wire it in `astro.config.mjs`** — add to the `starlight({...})` integration: `customCss: ['./src/styles/starlight.css']`; a `sidebar` with the six groups using `autogenerate: { directory: 'docs/<section>' }` and the exact labels from Global Constraints; set `trailingSlash: 'always'` at the Astro config top level for URL parity. Keep the existing placeholder doc building until Task 2 adds real content.

- [ ] **Step 3: Verify** — `pnpm --dir website build` succeeds; run `dev`, open `/docs/...`, and screenshot: the docs page uses warm paper, red accents, warm-dark code, and the six sidebar groups appear (empty/placeholder is fine now). Compare against the reference. Confirm light AND dark.
- [ ] **Step 4: Commit** — `git add website/src/styles/starlight.css website/astro.config.mjs && git commit -m "feat(docs): theme Starlight to the Loco warm identity + Diátaxis sidebar"`

---

### Task 2: Frontmatter conversion script + migrate all 64 docs

**Files:**
- Create: `website/scripts/convert-frontmatter.mjs`, `website/scripts/migrate-docs.mjs`, `website/scripts/scripts.test.mjs`
- Create (generated): `website/src/content/docs/docs/<section>/*.md` (64 files)
- Modify: `website/package.json` (add `smol-toml` devDependency + a `migrate:docs` script)

**Interfaces:**
- Produces: `convertFrontmatter(rawMarkdown: string): string` — replaces a leading Zola `+++ TOML +++` block with a Starlight `--- YAML ---` block per the mapping; leaves the body untouched. `migrate-docs.mjs` walks the source tree and writes converted files (mapping `_index.md`→`index.md`).

- [ ] **Step 1: Write the failing test** `scripts.test.mjs` for `convertFrontmatter`:

```js
import { describe, it, expect } from 'vitest';
import { convertFrontmatter } from './convert-frontmatter.mjs';

it('maps title/description/weight and drops zola-only keys', () => {
  const zola = `+++\ntitle = "Add a worker"\ndescription = "How to add a worker"\nsort_by = "weight"\nweight = 30\ntemplate = "docs/page.html"\n+++\n\n# Body\ntext {{ get_env(name='X') }}\n`;
  const out = convertFrontmatter(zola);
  expect(out).toMatch(/^---\n/);
  expect(out).toContain('title: Add a worker');
  expect(out).toContain('description: How to add a worker');
  expect(out).toContain('sidebar:\n  order: 30');
  expect(out).not.toContain('sort_by');
  expect(out).not.toContain('template');
  // body + literal config syntax preserved verbatim
  expect(out).toContain("text {{ get_env(name='X') }}");
});
it('quotes titles containing colons/special chars safely', () => {
  const zola = `+++\ntitle = "Loco: the tour"\nweight = 1\n+++\nbody\n`;
  expect(convertFrontmatter(zola)).toContain('title: "Loco: the tour"');
});
```

- [ ] **Step 2: Run test** — `cd website && pnpm add -D smol-toml && pnpm test` — Expected: FAIL (no `./convert-frontmatter.mjs`).
- [ ] **Step 3: Implement `convert-frontmatter.mjs`** — parse the leading `+++...+++` with `smol-toml`, build YAML with `title`, `description` (only if present), `sidebar: { order: weight }` (only if `weight` present); quote any string value containing `:`/`#`/leading special chars; drop all other keys; re-emit `---\n<yaml>---\n` + original body. Make it pure (string in → string out).
- [ ] **Step 4: Run test** — Expected: PASS (2 tests).
- [ ] **Step 5: Implement `migrate-docs.mjs`** — walk `docs-site/content/docs/**/*.md`; for each, `convertFrontmatter`, then write to `website/src/content/docs/docs/<relative path>`, renaming `_index.md`→`index.md`. Create section dirs as needed. Print a per-section count.
- [ ] **Step 6: Run the migration + build**

Run: `cd website && node scripts/migrate-docs.mjs && pnpm build`
Expected: prints tutorials 4 / how-to 35 / reference 11 / explanation 8 / extras 2 / resources 3 (+ index pages); build succeeds and every doc page renders (Starlight will warn on broken links — those are fixed in Task 3).

- [ ] **Step 7: Commit** — `git add website/scripts website/src/content/docs/docs website/package.json website/pnpm-lock.yaml && git commit -m "feat(docs): frontmatter conversion + migrate 64 Diátaxis docs to Starlight"`

---

### Task 3: Rewrite internal links to Starlight URLs

**Files:**
- Create: `website/scripts/rewrite-links.mjs`
- Modify: `website/scripts/scripts.test.mjs` (add cases); the 64 migrated `.md` files (in place)

**Interfaces:**
- Produces: `rewriteLinks(markdown: string): string` — turns `](@/docs/<path>.md)` and `](@/docs/<path>.md#anchor)` into `](/docs/<path>)` / `](/docs/<path>#anchor)`, leaving all other links untouched.

- [ ] **Step 1: Add failing tests** to `scripts.test.mjs`:

```js
import { rewriteLinks } from './rewrite-links.mjs';
it('rewrites @/docs links, preserving anchors, dropping .md', () => {
  expect(rewriteLinks('see [cfg](@/docs/reference/configuration.md)')).toBe('see [cfg](/docs/reference/configuration.md)'.replace('.md',''));
  expect(rewriteLinks('[m](@/docs/how-to/add-worker.md#queues)')).toBe('[m](/docs/how-to/add-worker#queues)');
});
it('leaves external and relative links alone', () => {
  const s = '[x](https://example.com) and [y](./local.md) and [z](#frag)';
  expect(rewriteLinks(s)).toBe(s);
});
```

- [ ] **Step 2: Run test** — Expected: FAIL (no `./rewrite-links.mjs`).
- [ ] **Step 3: Implement `rewrite-links.mjs`** — regex replace `\]\(@/docs/([^)#]+?)\.md(#[^)]*)?\)` → `](/docs/$1$2)`. Export `rewriteLinks`; also add a CLI mode that rewrites every file under `website/src/content/docs/docs/` in place.
- [ ] **Step 4: Run test** — Expected: PASS.
- [ ] **Step 5: Apply + verify no stragglers**

Run: `cd website && node scripts/rewrite-links.mjs --apply && grep -rn '@/' src/content/docs/docs || echo "NO @/ links remain"`
Expected: prints "NO @/ links remain"; then `pnpm build` succeeds with **zero broken-link warnings** for internal `/docs/...` targets (read the build output — a remaining warning is a defect to fix, usually a slug/case mismatch).

- [ ] **Step 6: Commit** — `git add website/scripts/rewrite-links.mjs website/scripts/scripts.test.mjs website/src/content/docs/docs && git commit -m "feat(docs): rewrite Zola @/ internal links to Starlight URLs"`

---

### Task 4: Sidebar order, section index pages, search — validation pass

**Files:**
- Modify: `website/astro.config.mjs` (only if sidebar ordering needs `order` tweaks); possibly section `index.md` frontmatter.

**Interfaces:**
- Consumes: Tasks 1–3 output.

- [ ] **Step 1:** Build + serve; verify in a real browser (headless Chrome screenshots) at desktop width, light AND dark: (a) the six sidebar groups render with the correct labels and their pages listed **in `weight`/`order`**; (b) a representative page from each section renders with warm theme, warm-dark code blocks, working on-page TOC; (c) **Pagefind search** (⌘K) returns results; (d) the version selector shows. Compare against `docs/superpowers/specs/2026-07-17-docs-starlight-reference.html`.
- [ ] **Step 2:** Fix any ordering/label/index-page issues found (e.g. a section whose `_index.md` weight didn't map, or a page out of order). Re-verify.
- [ ] **Step 3: Commit** (if changes) — `git commit -am "fix(docs): sidebar ordering + section index pages verified"`

---

### Task 5: llms.txt + URL parity + a11y/perf

**Files:**
- Modify: `website/astro.config.mjs` (add `starlight-llms-txt` plugin OR a generation step); create `website/public/llms.txt`/`llms-full.txt` if generated statically.
- Create: `website/scripts/url-parity.mjs` (compares old Zola doc URLs to new Starlight URLs).

**Interfaces:**
- Consumes: the full migrated docs.

- [ ] **Step 1: llms.txt** — add the `starlight-llms-txt` plugin (or a small build step) so `/llms.txt` and `/llms-full.txt` are emitted from the docs content. Verify `pnpm build` produces both in `dist/` and they contain real doc content.
- [ ] **Step 2: URL parity** — write `url-parity.mjs`: enumerate old doc slugs from `docs-site/content/docs/**` (path → `/docs/<section>/<slug>/`) and new ones from `website/dist/docs/**/index.html`; print any old URL with no new equivalent. Resolve mismatches (fix a slug, or add an Astro redirect in `astro.config.mjs`). Expected: every old `/docs/...` URL resolves in the new build.
- [ ] **Step 3: a11y/perf** — build; confirm docs pages have proper headings/landmarks (Starlight provides these), keyboard-navigable sidebar/search, and no framework runtime beyond Starlight's own. Spot-check one page with a headless-Chrome a11y snapshot.
- [ ] **Step 4: Commit** — `git add -A && git commit -m "feat(docs): llms.txt output, URL-parity check, a11y pass"`

---

## Follow-on (out of scope)

- **Blog + casts** migration (Astro content collections, RSS) — its own plan.
- **Cutover** — switch the deploy from `docs-site/` to `website/`, delete `docs-site/` — its own plan (after blog/casts).

## Self-Review

- **Spec coverage:** Starlight theming → Task 1; 64-file frontmatter conversion → Task 2; internal-link rewrite → Task 3; sidebar/search/version validation → Task 4; llms.txt + URL parity + a11y → Task 5. The spec's Docs requirements (Diátaxis sidebar, Pagefind, version selector, themed, llms.txt, URL parity) are all assigned.
- **Placeholder scan:** conversion/link scripts have concrete regexes + real tests; no "handle edge cases" hand-waving. The one judgment area (YAML quoting of special-char titles) has a dedicated test.
- **Type/name consistency:** `convertFrontmatter` (Task 2) and `rewriteLinks` (Task 3) are the two pure transforms, named identically in their defining and consuming steps. The nested `src/content/docs/docs/` path and `/docs/*` URL shape are stated in Global Constraints and reused by Tasks 2/3/5. Section labels are defined once and referenced by Tasks 1/4.
