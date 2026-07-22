# Loco Website — Foundation + Homepage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up the new Astro website project and build the Pure-Direction-A homepage to match the committed reference mockup, producing a deployable static site.

**Architecture:** One Astro project in `website/`, static output. Marketing pages are Astro components; Starlight is wired as an integration owning `/docs/*` (content migration is a later plan — here we only prove the route builds). A single `tokens.css` (light + dark) is the design system, consumed by Tailwind and later by Starlight. Homepage interactivity (copy, scroll-streaming terminal, parallax, slideshow) is one bundled vanilla-TS `<script>` — no framework runtime on marketing pages.

**Tech Stack:** Astro 5, Tailwind (via `@astrojs/tailwind`), `@astrojs/starlight`, `@astrojs/sitemap`, TypeScript, Vitest (for interaction logic), pnpm.

## Global Constraints

- **One Astro project** in `website/`; static output (`astro build`); do NOT modify or delete the existing `docs-site/` (cutover is a later plan).
- **Design tokens are the single source of truth** — light AND dark, both first-class. Light: `--paper:#FBF6EC; --paper-2:#F4ECDD; --card:#FFFDF8; --ink:#1C1712; --ink-2:#5A5148; --ink-3:#8A7F72; --line:#E7DDCB; --red:#FC3820; --red-ink:#D82A12`. Dark (warm espresso): `--paper:#17120E; --paper-2:#1F1813; --card:#221A14; --ink:#F2E9DC; --ink-2:#C3B6A5; --ink-3:#8E8071; --line:#33291F; --red:#FF5A3D; --red-ink:#FF7355`.
- **Positioning copy is fixed:** H1 is *"The one-person framework for Rust."*; the six pillars use the verbatim names/copy in the reference. Do not invent marketing copy.
- **All code shown on the page must be real, compiling Loco** — reproduce the exact snippets in the reference mockup; do not paraphrase APIs.
- **The pixel/behaviour reference is** `docs/superpowers/specs/2026-07-17-homepage-direction-a-reference.html` (self-contained). When a task says "port from the reference," open that file and copy the exact CSS/markup for the named block. It is the authoritative source for spacing, colour, and structure.
- **Explicitly rejected — do NOT add:** keyword marquee/auto-scroll, the top progress-rail "train", numbered "01 · Controller" eyebrows on the slideshow, any entrance/fall-in animation on the postcard stamps.
- **English only**; no framework runtime (React/Preact/Vue) on the homepage.
- The hero illustration is `media/image.png`; the logo mark is `docs-site/static/icon.svg`. Copy both into `website/public/` (do not hotlink out of the repo).

---

## File Structure

```
website/
  package.json, astro.config.mjs, tsconfig.json, vitest.config.ts
  public/
    loco-illustration.png        # from media/image.png
    loco-mark.svg                # from docs-site/static/icon.svg
    favicon-32x32.png, favicon-16x16.png, apple-touch-icon.png  # from docs-site/static
  src/
    styles/
      tokens.css                 # light + dark CSS custom properties (design system)
      global.css                 # reset-ish base, imports tokens, element defaults
    lib/
      slides.ts                  # SLIDES data + CODE map (the six parts, real Loco code)
      home.ts                    # homepage interactions (copy, terminal stream, parallax, deck)
      home.test.ts               # Vitest unit tests for the pure logic in home.ts/slides.ts
    components/
      Nav.astro
      Footer.astro
      ThemeToggle.astro
      PostcardFrame.astro        # framed illustration wrapper
      Pillar.astro               # one postcard pillar (icon + stamp + title + copy)
      CodeWindow.astro           # traffic-light bar + filename + copy + <pre>
    layouts/
      Base.astro                 # <html> shell, head, tokens/global css, theme init
    pages/
      index.astro                # the homepage, composed of the sections
```

Each Astro component owns one responsibility. `home.ts` holds every interaction; `slides.ts` holds the content/data so it is unit-testable without a DOM. Ordering of the six parts, all copy, and all code strings live in `slides.ts` (one source, reused by the deck).

---

### Task 1: Scaffold the Astro project and prove it builds

**Files:**
- Create: `website/package.json`, `website/astro.config.mjs`, `website/tsconfig.json`
- Create: `website/src/pages/index.astro` (temporary placeholder, replaced in Task 8)
- Create: `website/public/loco-illustration.png`, `website/public/loco-mark.svg`, favicons

**Interfaces:**
- Produces: a buildable Astro site; `pnpm --dir website build` emits `website/dist/`.

- [ ] **Step 1: Create `website/package.json`**

```json
{
  "name": "loco-website",
  "type": "module",
  "private": true,
  "scripts": {
    "dev": "astro dev",
    "build": "astro build",
    "preview": "astro preview",
    "check": "astro check",
    "test": "vitest run"
  },
  "dependencies": {
    "astro": "^5.0.0",
    "@astrojs/tailwind": "^5.1.0",
    "@astrojs/starlight": "^0.30.0",
    "@astrojs/sitemap": "^3.0.0",
    "tailwindcss": "^3.4.0"
  },
  "devDependencies": {
    "vitest": "^2.0.0",
    "jsdom": "^25.0.0",
    "@astrojs/check": "^0.9.0",
    "typescript": "^5.5.0"
  }
}
```

- [ ] **Step 2: Create `website/astro.config.mjs`** (Starlight owns `/docs`; content is a later plan, so point it at an empty sidebar for now)

```js
import { defineConfig } from 'astro/config';
import tailwind from '@astrojs/tailwind';
import starlight from '@astrojs/starlight';
import sitemap from '@astrojs/sitemap';

export default defineConfig({
  site: 'https://loco.rs',
  integrations: [
    tailwind({ applyBaseStyles: false }),
    starlight({
      title: 'Loco',
      // Docs content is migrated in a later plan; keep the route buildable.
      sidebar: [{ label: 'Docs', items: [{ label: 'Overview', slug: 'index' }] }],
    }),
    sitemap(),
  ],
});
```

- [ ] **Step 3: Create `website/tsconfig.json`**

```json
{ "extends": "astro/tsconfigs/strict", "include": [".astro", "src"] }
```

- [ ] **Step 4: Add a Starlight placeholder doc so the build has content**

Create `website/src/content/docs/index.md`:
```md
---
title: Loco Docs
description: Documentation home (content migrated in a later plan).
---
Docs content is migrated in a later plan.
```

- [ ] **Step 5: Temporary homepage placeholder**

Create `website/src/pages/index.astro`:
```astro
---
---
<html lang="en"><head><meta charset="utf-8"><title>Loco</title></head>
<body><h1>Loco — placeholder</h1></body></html>
```

- [ ] **Step 6: Copy binary assets into `public/`**

```bash
mkdir -p website/public
cp media/image.png website/public/loco-illustration.png
cp docs-site/static/icon.svg website/public/loco-mark.svg
cp docs-site/static/favicon-32x32.png docs-site/static/favicon-16x16.png docs-site/static/apple-touch-icon.png website/public/
```

- [ ] **Step 7: Install and build**

Run: `cd website && pnpm install && pnpm build`
Expected: build succeeds; `website/dist/index.html` and `website/dist/docs/index.html` exist.

- [ ] **Step 8: Commit**

```bash
git add website/package.json website/astro.config.mjs website/tsconfig.json website/src website/public
git commit -m "feat(website): scaffold Astro project with Starlight/docs route and assets"
```

---

### Task 2: Design tokens (light + dark) and the theme toggle mechanism

**Files:**
- Create: `website/src/styles/tokens.css`, `website/src/styles/global.css`
- Create: `website/src/layouts/Base.astro`, `website/src/components/ThemeToggle.astro`
- Modify: `website/src/pages/index.astro` (use Base layout to verify tokens render)

**Interfaces:**
- Produces: `Base.astro` (a layout accepting `title`, `description` props and a default slot; injects tokens/global CSS and the no-flash theme init). `ThemeToggle.astro` (a `<button class="toggle">` that flips `data-theme` on `:root` and persists to `localStorage`).

- [ ] **Step 1: Write `tokens.css`** — the two palettes as custom properties.

```css
:root {
  --paper:#FBF6EC; --paper-2:#F4ECDD; --card:#FFFDF8;
  --ink:#1C1712; --ink-2:#5A5148; --ink-3:#8A7F72; --line:#E7DDCB;
  --red:#FC3820; --red-ink:#D82A12;
  --sans:-apple-system,BlinkMacSystemFont,"Segoe UI",Inter,Roboto,system-ui,sans-serif;
  --mono:ui-monospace,"SF Mono","JetBrains Mono",Menlo,monospace;
}
:root[data-theme="dark"] {
  --paper:#17120E; --paper-2:#1F1813; --card:#221A14;
  --ink:#F2E9DC; --ink-2:#C3B6A5; --ink-3:#8E8071; --line:#33291F;
  --red:#FF5A3D; --red-ink:#FF7355;
}
@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) {
    --paper:#17120E; --paper-2:#1F1813; --card:#221A14;
    --ink:#F2E9DC; --ink-2:#C3B6A5; --ink-3:#8E8071; --line:#33291F;
    --red:#FF5A3D; --red-ink:#FF7355;
  }
}
```

- [ ] **Step 2: Write `global.css`** — import tokens, base element defaults (margin reset, body uses `--paper`/`--ink`/`--sans`, `a{color:inherit;text-decoration:none}`, `*{box-sizing:border-box}`). Port the base rules from the reference mockup's `<style>` preamble.

- [ ] **Step 3: Write `Base.astro`** with a no-flash theme script in `<head>` (runs before paint):

```astro
---
const { title = 'Loco — the one-person framework for Rust', description = '' } = Astro.props;
import '../styles/global.css';
---
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{title}</title>
  <meta name="description" content={description} />
  <link rel="icon" href="/favicon-32x32.png" />
  <script is:inline>
    const t = localStorage.getItem('theme');
    if (t) document.documentElement.setAttribute('data-theme', t);
  </script>
</head>
<body><slot /></body>
</html>
```

- [ ] **Step 4: Write `ThemeToggle.astro`**

```astro
<button class="toggle" aria-label="Toggle color theme" data-theme-toggle>◐</button>
<script>
  document.querySelector('[data-theme-toggle]')?.addEventListener('click', () => {
    const root = document.documentElement;
    const next = root.getAttribute('data-theme') === 'dark' ? 'light' : 'dark';
    root.setAttribute('data-theme', next);
    localStorage.setItem('theme', next);
  });
</script>
```

- [ ] **Step 5: Verify** — point `index.astro` at `Base` with a `<ThemeToggle/>` and a swatch div using `background:var(--paper)`. Run `pnpm --dir website build` (expect success) and `pnpm --dir website dev`, confirm the toggle flips light/dark and persists on reload.

- [ ] **Step 6: Commit** — `git add website/src/styles website/src/layouts website/src/components/ThemeToggle.astro website/src/pages/index.astro && git commit -m "feat(website): light+dark design tokens, Base layout, theme toggle"`

---

### Task 3: Slide data + code map (`slides.ts`) with unit tests

**Files:**
- Create: `website/src/lib/slides.ts`, `website/src/lib/home.test.ts`, `website/vitest.config.ts`

**Interfaces:**
- Produces:
  - `export interface Slide { k: string; title: string; bullets: string[] }`
  - `export const SLIDES: Slide[]` — six entries in order: `controller, model, worker, view, task, mailer`.
  - `export interface CodeEntry { file: string; html: string }`
  - `export const CODE: Record<string, CodeEntry>` — keyed by `Slide.k`.
  - `export function stripTags(html: string): string` — returns the plain code text (used by the copy button and tests).

- [ ] **Step 1: Write the failing test** `home.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { SLIDES, CODE, stripTags } from './slides';

describe('slides data', () => {
  it('has the six parts in order', () => {
    expect(SLIDES.map(s => s.k)).toEqual(['controller','model','worker','view','task','mailer']);
  });
  it('every slide has a matching real code entry with a filename and 3 bullets', () => {
    for (const s of SLIDES) {
      expect(CODE[s.k]).toBeTruthy();
      expect(CODE[s.k].file).toMatch(/^src\/.+\.rs$/);
      expect(s.bullets.length).toBe(3);
    }
  });
  it('code is real Loco: controller references the model and view', () => {
    const plain = stripTags(CODE.controller.html);
    expect(plain).toContain('articles::Model::latest');
    expect(plain).toContain('ArticleResponse');
    expect(plain).toContain('Routes::new()');
  });
  it('stripTags removes span markup but keeps code text', () => {
    expect(stripTags('<span class="k">use</span> loco;')).toBe('use loco;');
  });
});
```

- [ ] **Step 2: Create `vitest.config.ts`** (`environment: 'node'`) and run the test.
Run: `cd website && pnpm test`
Expected: FAIL — cannot find `./slides`.

- [ ] **Step 3: Implement `slides.ts`** — port the `SLIDES` array and `CODE` map verbatim from the reference mockup's `<script>` (the `SLIDES=[…]` and `CODE={…}` literals) into typed exports, and add:

```ts
export function stripTags(html: string): string {
  return html.replace(/<[^>]+>/g, '')
    .replace(/&lt;/g,'<').replace(/&gt;/g,'>').replace(/&amp;/g,'&');
}
```

The six `CODE` entries are the exact controller/model/worker/view/task/mailer snippets in the reference — copy them character-for-character (they are real, compiling Loco).

- [ ] **Step 4: Run the tests.**
Run: `cd website && pnpm test`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit** — `git add website/src/lib/slides.ts website/src/lib/home.test.ts website/vitest.config.ts && git commit -m "feat(website): six-part slide data + real Loco code map with tests"`

---

### Task 4: Homepage interaction logic (`home.ts`) with unit tests

**Files:**
- Create: `website/src/lib/home.ts`
- Modify: `website/src/lib/home.test.ts` (add cases)

**Interfaces:**
- Produces (pure, DOM-free helpers — the DOM wiring is a thin `initHome()` that calls these):
  - `export function terminalHtml(revealChars: number): string` — given a character count, returns the streamed terminal markup (prompt-coloured spans + trailing cursor). Segments identical to the reference `seg` array.
  - `export const TERMINAL_TOTAL: number` — total character count of the terminal script.
  - `export function streamProgress(scrollY: number, offset = 128, span = 640): number` — clamped 0..1 (matches reference).
  - `export function wrapIndex(i: number, len: number): number` — modular slide index.
  - `export function initHome(doc: Document): void` — wires copy buttons, scroll-stream terminal, parallax, and the slideshow deck onto a document (no-ops if elements absent).

- [ ] **Step 1: Add failing tests** to `home.test.ts`:

```ts
import { terminalHtml, streamProgress, wrapIndex, TERMINAL_TOTAL } from './home';

describe('terminal streaming', () => {
  it('reveals nothing but a cursor at 0 chars', () => {
    expect(terminalHtml(0)).toBe('<span class="cur"></span>');
  });
  it('reveals the whole first line at the rest offset', () => {
    // (128/640)*TERMINAL_TOTAL rounds to the first prompt line
    const html = terminalHtml(Math.round((128/640)*TERMINAL_TOTAL));
    expect(html).toContain('loco new saas_app');
    expect(html).not.toContain('listening on');
  });
  it('streamProgress clamps to 0..1', () => {
    expect(streamProgress(-9999)).toBe(0);
    expect(streamProgress(99999)).toBe(1);
  });
});
describe('deck index', () => {
  it('wraps around both directions', () => {
    expect(wrapIndex(-1, 6)).toBe(5);
    expect(wrapIndex(6, 6)).toBe(0);
  });
});
```

- [ ] **Step 2: Run tests.** Run: `cd website && pnpm test` — Expected: FAIL (no `./home`).

- [ ] **Step 3: Implement `home.ts`** — port the reference `<script>` logic into these functions. `terminalHtml`/`TERMINAL_TOTAL` from the reference `seg`/`renderTerm`; `streamProgress` from `(y+128)/640` clamp; `wrapIndex` from `(i+len)%len`. Then `initHome(doc)` wires: `[data-copy]` buttons (copying literal value, or the active deck code via `stripTags`), the scroll handler (rAF-throttled) driving `terminalHtml` + illustration/terminal parallax transforms, and the deck (`paint(i)` updates `#dtitle`/`#dbul`/`#dcode`/`#dfile`/dots/`#dnum`; prev/next/dots call `go`). Copy exact transform expressions and rootMargins from the reference.

- [ ] **Step 4: Run tests.** Run: `cd website && pnpm test` — Expected: PASS (all).

- [ ] **Step 5: Commit** — `git add website/src/lib/home.ts website/src/lib/home.test.ts && git commit -m "feat(website): homepage interaction logic (terminal stream, parallax, deck) with tests"`

---

### Task 5: Nav, Footer, ThemeToggle wiring, PostcardFrame, CodeWindow components

**Files:**
- Create: `website/src/components/Nav.astro`, `Footer.astro`, `PostcardFrame.astro`, `CodeWindow.astro`

**Interfaces:**
- Consumes: `ThemeToggle.astro` (Task 2).
- Produces:
  - `Nav.astro` — sticky blurred nav: mark + `loco`, links, `★ 6.9k`, `<ThemeToggle/>`, primary CTA. Port markup+styles from the reference `<nav>`.
  - `Footer.astro` — port the reference `<footer>`.
  - `PostcardFrame.astro` — slot-based framed card (rotation, shadow, optional caption prop). Wraps the hero illustration.
  - `CodeWindow.astro` — props `{ file: string }`, slot for `<pre>` content; renders the traffic-light bar + filename + a `data-copy` button. Used by the deck (Task 7).

- [ ] **Step 1:** Implement the four components, porting exact CSS/markup from the correspondingly-named blocks in the reference file. Keep each component's `<style>` scoped; shared tokens come from `var(--…)`.
- [ ] **Step 2: Verify build** — temporarily render all four in `index.astro`. Run `pnpm --dir website build` (expect success) and eyeball in `dev`.
- [ ] **Step 3: Commit** — `git add website/src/components && git commit -m "feat(website): Nav, Footer, PostcardFrame, CodeWindow components"`

---

### Task 6: Hero + keyword strip

**Files:**
- Modify: `website/src/pages/index.astro` (add hero + strip sections)
- Create: `website/src/components/Hero.astro`

**Interfaces:**
- Consumes: `PostcardFrame`, and (later) `home.ts` `initHome` for the streaming terminal.
- Produces: the hero markup with the exact IDs `home.ts` expects — `#term` (the `<pre class="codebody">`), the `.art .frame`/`.art .code` structure, the `data-copy="cargo install loco"` install chip, and the animated `.u` underline on "one-person".

- [ ] **Step 1:** Build `Hero.astro` porting the reference hero: eyebrow pill, H1 with `.u` underline, lede, CTA + install chip (`data-copy`), trust row, and the `PostcardFrame` with the illustration + overlapping terminal card (`#term`). Give the terminal `<pre>` the reference's **fixed height** (`height:104px;overflow:hidden`) so streaming can't grow it.
- [ ] **Step 2:** Add the **static** keyword strip below the hero (no marquee) — port the `.strip .row` markup.
- [ ] **Step 3: Verify** — build + dev. The terminal will be empty until Task 8 wires `initHome`; assert the layout matches the reference hero and the install chip renders.
- [ ] **Step 4: Commit** — `git add website/src/components/Hero.astro website/src/pages/index.astro && git commit -m "feat(website): hero section + static keyword strip"`

---

### Task 7: Pillars (postcards + distressed stamp) and the slideshow deck

**Files:**
- Create: `website/src/components/Pillars.astro`, `website/src/components/Deck.astro`
- Modify: `website/src/pages/index.astro`

**Interfaces:**
- Consumes: `slides.ts` (`SLIDES`, `CODE`), `CodeWindow.astro`.
- Produces:
  - `Pillars.astro` — the six postcards (icon, title, dashed rule, copy) each with the `.applied` logo stamp. **Includes the inline `<svg><filter id="stampInk">…</filter></svg>`** (rough displacement + speckled `feComponentTransfer` mask) once on the page; the stamp is `filter:grayscale(1) contrast(1.15) url(#stampInk); mix-blend-mode:multiply` with the crushed-ring `::before`, sized/positioned to **clip at the card's top-right corner**. No entrance animation.
  - `Deck.astro` — the fixed-height panel: left `.deck-copy` (h3 `#dtitle`, `<ul id="dbul">`, and the bottom-pinned control `.deck-ctrl` with `#dprev`/`#dnext`/`#ddots`/`#dnum`), right a `CodeWindow` with `<pre id="dcode" class="bigcode">`. **No numbered eyebrow.** Fixed panel height + `minmax(0,…)` columns + top-aligned copy + `margin-top:auto` control, exactly as the corrected reference.

- [ ] **Step 1:** Implement `Pillars.astro` porting the reference `.pillars`/`.pc`/`.applied`/`#stampInk` blocks verbatim. Render the six pillars from an inline array of `{icon,title,copy}` (copy verbatim from the reference).
- [ ] **Step 2:** Implement `Deck.astro` porting the reference `.deck-*` blocks (post-fix versions: fixed height `524px`, `justify-content:flex-start`, control `margin-top:auto`, code `font-size:12px;line-height:1.72`). The deck renders the slide-0 content server-side; `home.ts` hydrates and swaps on interaction.
- [ ] **Step 3: Verify** — build + dev. Confirm: stamp clips at the corner and looks distressed; deck panel height/split identical across slides once wired.
- [ ] **Step 4: Commit** — `git add website/src/components/Pillars.astro website/src/components/Deck.astro website/src/pages/index.astro && git commit -m "feat(website): postcard pillars with rubber-stamp mark + six-part slideshow deck"`

---

### Task 8: Assemble the homepage, wire interactions, a11y + perf pass

**Files:**
- Modify: `website/src/pages/index.astro` (final composition + script)
- Create: `website/src/lib/home.entry.ts` (imports `initHome` and calls it on `DOMContentLoaded`)

**Interfaces:**
- Consumes: all components + `home.ts`.

- [ ] **Step 1:** Compose `index.astro` with `Base` → `Nav`, `Hero`, keyword strip, `Pillars`, `Deck` (header "One feature, a few small files."), `Footer`. Add `<script>import { initHome } from '../lib/home'; initHome(document);</script>` (Astro bundles it; runs on the client).
- [ ] **Step 2: Verify interactions in `dev`** — copy buttons work; the terminal streams on scroll and never grows past its fixed height; parallax drifts; the deck pages via arrows/dots with fixed height + fixed split and no code clipping; theme toggle flips both hero and deck.
- [ ] **Step 3: a11y + perf** — buttons have `aria-label`s; keyboard focus visible; run `pnpm --dir website build` and confirm no framework runtime is shipped (only the small `home` script). Add per-page `<title>`/description/OG in `Base`.
- [ ] **Step 4: Screenshot-verify against the reference** — build, serve `dist/`, and compare the rendered homepage to `2026-07-17-homepage-direction-a-reference.html` at desktop width; differences are defects.
- [ ] **Step 5: Commit** — `git add website/src/pages/index.astro website/src/lib/home.entry.ts && git commit -m "feat(website): assemble Pure-A homepage with wired interactions"`

---

## Follow-on plans (out of scope here)

These become their own plans, each shipping independently:
- **Docs migration** — convert the 83 Diátaxis markdown files (Zola TOML frontmatter + shortcodes → Starlight), theme Starlight to the tokens, Pagefind search, version selector, `llms.txt`.
- **Blog + casts** — Astro content collections, author pages, RSS at the existing feed URLs.
- **Cutover** — SEO/sitemap/redirects, URL-parity audit, a11y/perf, switch the deploy, delete `docs-site/`.

## Self-Review

- **Spec coverage:** Foundation (Astro+Starlight+Tailwind, tokens light+dark, shared components, theme toggle) → Tasks 1–2, 5. Homepage Pure-A (hero+streaming terminal, keyword strip, postcard pillars+stamp, six-part slideshow, footer, motion) → Tasks 3–8. Docs/blog/cutover → explicitly deferred to follow-on plans. No spec homepage requirement is unassigned.
- **Rejected-items guard:** the Global Constraints list the four rejected elements; Tasks 6–7 restate "no marquee / no eyebrow / no stamp animation."
- **Type consistency:** `Slide.k` keys (`controller…mailer`) are the shared join between `SLIDES`, `CODE`, and the deck; `stripTags`, `terminalHtml`, `TERMINAL_TOTAL`, `streamProgress`, `wrapIndex`, `initHome` are named identically in their defining task (3/4) and consuming task (8). DOM IDs (`#term`, `#dtitle`, `#dbul`, `#dcode`, `#dfile`, `#ddots`, `#dnum`, `#dprev`, `#dnext`) are defined in Tasks 6–7 and consumed by `initHome` in Task 4/8.
