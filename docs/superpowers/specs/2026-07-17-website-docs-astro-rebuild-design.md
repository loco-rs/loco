# Loco Website + Docs Rebuild — Design Spec

**Date:** 2026-07-17
**Status:** Approved design, ready for implementation planning
**Supersedes:** the current Zola site in `docs-site/`

## Goal

Rebuild loco.rs — marketing site **and** documentation — on **Astro + Starlight**,
with a fresh, serious, warm visual identity ("Direction A") that keeps Loco's
existing brand (the hand-drawn locomotive, `#FC3820`, the "one-person framework"
positioning) but expresses it with real craft. Docs and marketing must feel like
one product.

## Why (drivers, from brainstorming)

1. **Look & feel is dated** — need a modern, distinctive identity.
2. **Zola is limiting** — no components-in-markdown (MDX), awkward theming, hard to
   build richer marketing sections.
3. **Docs UX specifically** — the reading experience must be best-in-class
   (sidebar, search, TOC, versioning), competitive with Rails/Next.js docs.

Authoring/maintenance friction was explicitly **not** a driver — the content model
is fine; the presentation layer and docs engine are what change.

## Locked decisions

| Decision | Choice |
|---|---|
| Platform | **Astro** (single project) |
| Docs engine | **Starlight** integration, mounted at `/docs/*` |
| Homepage direction | **Pure Direction A** — illustration-led warmth, calm (no interactive stations, no IDE/metrics blocks) |
| Docs theming | Starlight themed into the Direction A identity |
| Color mode | **Light + dark** (warm-dark, not cold black) |
| i18n | **English-only, i18n-ready**; drop the stale `tour-fr.md` |
| Repo layout | Build fresh in `website/`, cut over at parity, then delete `docs-site/` |
| Output | Static (`astro build`), same hosting model as today |

## Architecture

**One Astro project.** Marketing pages are Astro pages/components; Starlight owns
`/docs/*` as an integration in the same project. One repo, one build, one deploy,
one shared token file — so homepage and docs cannot drift.

```
website/
  astro.config.mjs         # Astro + Starlight + tailwind + sitemap + rss integrations
  src/
    styles/
      tokens.css           # THE design system: light + dark tokens (single source)
      starlight.css        # Starlight CSS-variable overrides → maps tokens onto Starlight
    components/            # shared, used by marketing AND docs overrides
      Nav.astro  Footer.astro  CodeBlock.astro  Callout.astro
      PostcardFrame.astro  ThemeToggle.astro  CopyButton.astro
    pages/
      index.astro          # Pure Direction A homepage
      blog/[...].astro      # blog index + posts (content collection)
      casts/[...].astro     # screencasts (content collection)
      privacy-policy.astro
      rss.xml.ts           # @astrojs/rss feed
    content/
      docs/                # Starlight docs (migrated 83 files, Diátaxis)
      blog/                # migrated blog posts
      casts/               # migrated screencasts
    content.config.ts      # collections schema (blog, casts; docs uses Starlight loader)
  public/                  # favicons, illustration, mark, scenery svgs, llms.txt, robots
```

**Cutover:** develop `website/` in parallel; keep `docs-site/` serving until
`website/` reaches parity (all URLs, feeds, redirects verified); switch the deploy;
delete `docs-site/` in a follow-up commit.

## Design system

Single source of truth: `src/styles/tokens.css`, consumed by Tailwind (marketing)
and by `starlight.css` (docs). Both light and dark are first-class.

**Light (primary identity):**
```
--paper:#FBF6EC; --paper-2:#F4ECDD; --card:#FFFDF8;
--ink:#1C1712; --ink-2:#5A5148; --ink-3:#8A7F72; --line:#E7DDCB;
--red:#FC3820; --red-ink:#D82A12;
```

**Dark (warm espresso, NOT cold black):**
```
--paper:#17120E; --paper-2:#1F1813; --card:#221A14;
--ink:#F2E9DC; --ink-2:#C3B6A5; --ink-3:#8E8071; --line:#33291F;
--red:#FF5A3D; --red-ink:#FF7355;   /* brightened for AA contrast on dark */
```

- **Type:** system sans stack (Inter/system-ui) for UI/body; large, tight-tracked,
  heavy weight for display headlines. One modular scale.
- **Code theme:** the warm-dark palette used in the mockups (`#1B1512` bg, coral
  keywords, green strings) — identical in light and dark modes, so code always has
  contrast against warm paper.
- **The illustration** (`media/image.png`, the hand-drawn locomotive) is a hero
  asset, always framed in a `--card` container (`PostcardFrame`) — in dark it reads
  as a warm print on espresso, which is intentional and attractive. Never reduced
  to an emoji.
- **Shared components** (see tree): Nav, Footer, CodeBlock (+copy), Callout,
  PostcardFrame, ThemeToggle. Marketing and docs use the SAME components so the two
  surfaces stay identical.

## Homepage (Pure Direction A) — validated design

Calm, confident, illustration-led. **Pixel reference:** a full interactive mockup
is committed alongside this spec at
`2026-07-17-homepage-direction-a-reference.html` (self-contained, embedded assets).
The implementer should reproduce its look and behavior in Astro components; details
below are the binding spec, the mockup is the visual source of truth.

Section order, top to bottom:

1. **Nav** (sticky, blurred paper bg) — mark + `loco` wordmark; links (Docs, Guides,
   Blog, Screencasts, Playground); GitHub star count; theme toggle; primary CTA.
2. **Hero** (two-column) — eyebrow pill ("Batteries-included Rust · v1.0"); H1 *"The
   **one-person** framework for Rust."* with an animated hand-drawn underline on
   "one-person"; lede; primary CTA + a **copyable** `cargo install loco`; a small
   trust row (version, stars, "0 → prod"). Right: the framed illustration
   (`PostcardFrame`, slight rotation, photo shadow, "// all aboard" caption) with a
   dark **terminal card overlapping its bottom-right**. The terminal has a **fixed
   height** and its text **streams as the user scrolls** (`loco new → cargo loco
   start`) — it must never grow/reflow the card as lines appear.
3. **Keyword strip** — static, understated: batteries-included · Rails, in Rust ·
   SeaORM · Axum · background jobs · test-driven. (No marquee/auto-scroll — rejected.)
4. **Pillars** — the real six, verbatim (Batteries included, Rails is great, Deliver
   with confidence, Scale when needed, Build incrementally, Test-driven everything),
   as **postcards**: warm card, dashed inner frame, a dashed-square feature icon,
   slight alternating tilt that straightens on hover. Each postcard carries a
   **rubber-stamp mark in its top-right corner**: a grayscale silhouette of the
   real logo (`grayscale` + `mix-blend-mode: multiply` so the white drops out),
   distressed via an SVG ink filter (rough displacement + speckled gaps),
   **bordered by a crushed circular ring**, sized so it **clips against the card's
   top-right corner** like an affixed stamp. Static (no entrance animation).
5. **"One feature, a few small files"** — a **slideshow** (not a scroll-through)
   cycling the six parts of a Loco feature: **Controller → Model → Worker → View →
   Task → Mailer**. Layout per slide: **left** = title + 3 value bullets (bold lead
   + detail, red check, inline code chips); **right** = a code window with **real,
   cross-referencing Loco source** for that part (controller calls
   `Model::latest()` + `ArticleResponse::new()`; view is the serializer; worker is
   `BackgroundWorker`; task is `impl Task`; mailer is `impl Mailer` + `mail_template`).
   Controls: compact **prev/next + dot indicators + `NN / 06` counter**, pinned to
   the **bottom-left** of the panel. Requirements: **fixed panel height** and
   **fixed text/code split width** across all slides (no jump); copy content is
   **top-aligned** (titles never shift); code font sized so no slide clips
   horizontally or vertically. **No numbered eyebrow** ("01 · Controller" style) —
   rejected.
6. **Footer.**

**Motion (tasteful, calm):** copy-to-clipboard on install + code; scroll-streaming
terminal; animated headline underline on load; scroll-reveal fade-ups; light
hero parallax (illustration + terminal drift at different rates, subtle 3D tilt on
the illustration). No progress-rail/train gimmick, no keyword marquee (both
explicitly rejected). All motion is vanilla JS / CSS in the mockup and maps to a
small Astro client script island.

Copy is drawn from the current live site's proven positioning, not invented. All
code shown is real, compiling Loco.

## Docs (Starlight, themed)

- **Layout:** three-pane — left sidebar (Diátaxis groups), main, right "On this
  page" TOC. Prev/next pager.
- **Sidebar structure** mirrors the existing content: `Tutorials`, `How-to guides`,
  `Reference`, `Explanation`, plus `Extras`/`Resources` as today.
- **Search:** Starlight's built-in **Pagefind** (⌘K).
- **Version selector** in the top bar — single version now (`v1.0`), wired so future
  versions can be added (via a Starlight versions plugin or manual version routing)
  without re-architecting.
- **Theming:** `starlight.css` overrides Starlight's CSS custom properties with our
  tokens — warm paper background, `#FC3820` accents on active nav / links / inline
  code / callouts, warm-dark code blocks. Light + dark both themed.
- **Preserve** `llms.txt` / `llms-full.txt` generation (AI-consumable docs) — keep
  this capability in the new build.

## Blog & Screencasts

- Astro **content collections** (`content/blog`, `content/casts`) with typed
  frontmatter schemas in `content.config.ts`.
- Blog authors: port the current authors taxonomy as a frontmatter field +
  author pages (no need for Zola's taxonomy engine).
- **RSS/Atom** via `@astrojs/rss`, preserving existing feed URLs
  (`blog/atom.xml`, `blog/rss.xml`) to avoid breaking subscribers.
- Restyle blog/casts shells into the new identity; **content ports as-is.**

## Content migration (largest, mostly mechanical)

- **83 Markdown files.** Convert Zola TOML frontmatter (`+++ … +++`) → Starlight/
  Astro YAML frontmatter (title, description, sidebar order/label, etc.).
- Replace Zola constructs: the `get_env` shortcode and `{% raw %}…{% endraw %}`
  blocks → MDX components or plain text. Inventory every shortcode use first.
- **URL parity:** preserve existing doc/blog/cast paths, or add redirects. Produce a
  before/after URL map; verify no live URL 404s.
- Approach: a conversion script for the frontmatter/bulk transforms, then a manual
  review pass per file (prose is fine; frontmatter + embedded shortcodes are the
  risk). Migrate section-by-section (Tutorials first) to validate the pipeline early.

## Interactivity (minimal — Pure A is calm)

Astro is static-first; ship almost no JS. Islands only where they earn it:
- **CopyButton** on code blocks and the install command.
- **ThemeToggle** (light/dark), persisting choice, respecting `prefers-color-scheme`.
- **Mobile nav** toggle.

No marketing-page framework runtime (no React on the marketing side) — Astro
components + tiny vanilla scripts. (The separate `examples/reference_spa` React app
is unrelated to this site.)

## Cross-cutting

- **SEO/meta:** per-page title/description/OG; sitemap via `@astrojs/sitemap`;
  `robots.txt`; canonical URLs.
- **Assets:** carry over favicons, `apple-touch-icon`, the mark (`icon.svg`), the
  illustration, scenery SVGs (`mountain-bg`, `cloud`, `top`), benchmark SVGs
  (`bench-db-q`, `bench-no-db`) for a future why/benchmarks page.
- **Accessibility:** verify AA contrast in both themes (esp. `#FC3820` on paper and
  the brightened red on espresso); keyboard-navigable nav/search; focus states.
- **Performance:** static output, no heavy JS, optimized images (Astro `<Image>`).

## Build & deploy

- `astro build` → static site. Keep the current host/CI target; swap the build
  command from Zola to Astro. Node toolchain for the site (kept out of the Rust
  workspace's concerns; `website/` is self-contained with its own `package.json`).

## Suggested implementation phases (for the plan)

1. **Foundation** — scaffold `website/` (Astro + Starlight + Tailwind), `tokens.css`
   (light+dark), shared components, theme toggle. Deliverable: themed empty shell.
2. **Docs migration** — frontmatter/shortcode conversion pipeline; migrate Diátaxis
   sections; Starlight theming (`starlight.css`); Pagefind search; version selector;
   `llms.txt`. Deliverable: full docs at parity.
3. **Homepage** — Pure Direction A, real content, both themes.
4. **Blog + casts** — content collections, author pages, RSS at existing URLs.
5. **Cross-cutting + cutover** — SEO/sitemap/redirects, URL-parity audit, a11y +
   perf pass, switch deploy, delete `docs-site/`.

## Out of scope / deferred

- Additional locales / translations (i18n stays wired but unused).
- Interactive homepage stations and IDE/metrics blocks (Directions B/C — explicitly
  set aside; can be revisited later).
- Multi-version docs content (structure is versioning-ready; only v1.0 ships now).
- A dedicated benchmarks/comparison page (assets preserved; not built now).

## Risks

- **Content migration is the bulk** and the main risk — shortcode edge cases and URL
  parity. Mitigate with an early end-to-end pipeline test on one section and a URL
  map.
- **Dark theme doubles theming surface** — the warm identity doesn't translate to
  dark trivially. Mitigate by designing dark tokens up front (done above) and
  reviewing both modes per component.
- **Feed/URL breakage** — preserve existing feed and page URLs; audit before cutover.
