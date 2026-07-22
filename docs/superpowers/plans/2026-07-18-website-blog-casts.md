# Loco Website — Blog + Casts Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate the blog (5 posts), screencasts (7 casts), and authors (2) from Zola into the Astro site as content collections, styled in the warm Loco identity, with RSS feeds at the existing URLs.

**Architecture:** Astro content collections (`blog`, `casts`, `authors`) with typed Zod schemas. Custom Astro pages (NOT Starlight — these are marketing surfaces) reusing the homepage's `Base`/`Nav`/`Footer`/`CodeWindow` components and the design tokens. Markdown bodies render via Astro's built-in `render()` with the warm typography + warm-dark code. RSS via `@astrojs/rss` at `/blog/rss.xml` and `/blog/atom.xml`.

**Tech Stack:** Astro 5 content collections, `@astrojs/rss`, `smol-toml`, Vitest, pnpm.

## Global Constraints

- **Source is read-only:** Zola content in `docs-site/content/{blog,casts,authors}/` — do NOT modify `docs-site/`. Output goes to `website/src/content/{blog,casts,authors}/` and `website/src/pages/{blog,casts,authors}/`.
- **Warm identity, reuse existing components:** pages use the `Base` layout, `Nav`, `Footer`, and design tokens (`website/src/styles/tokens.css`) — the same warm-postcard look as the homepage (`docs/superpowers/specs/2026-07-17-homepage-direction-a-reference.html`). Code blocks in posts use the warm-dark palette (match the docs/homepage). Both light and dark themes.
- **Frontmatter mapping:** Zola TOML → collection frontmatter. Blog: `title`, `description`, `date`→`pubDate` (Date), `updated`→`updatedDate` (optional), `taxonomies.authors`→`authors: string[]` (author slugs, lowercased/kebab of the taxonomy value), skip `draft = true`. Casts: same + `extra.num`→`episode` (string), `extra.id`→`youtube` (string, the video id). Authors: `title`→`name`, `description`, body→bio (markdown). Drop `template`/`sort_by`/`paginate_by`/`draft=false`.
- **URL parity (preserve, trailing slash):** posts `/blog/<slug>/`, casts `/casts/<slug>/`, authors `/authors/<slug>/`, blog index `/blog/`, casts index `/casts/`. Existing slugs — blog: `angular-frontend, hello-world, deploy-aws, axum-session, frontend-website`; casts: `001-…`–`007-…`. Author slugs: `limpidcrypto`, `team-loco`.
- **Feeds:** RSS at `/blog/rss.xml` AND `/blog/atom.xml` (both existed) with the blog posts (title, description, link, pubDate).
- **Author taxonomy values → slugs:** the Zola taxonomy uses display names (`"LimpidCrypto"`, `"Team Loco"`); map to the author file slugs (`limpidcrypto`, `team-loco`) via lowercase + spaces→hyphens. A post's `authors` array holds these slugs.
- English only. No framework runtime (plain Astro).

---

## File Structure

```
website/
  src/content.config.ts                 # MODIFY: add blog/casts/authors collections + Zod schemas
  scripts/migrate-blog-casts.mjs        # CREATE: Zola TOML → collection frontmatter for blog/casts/authors
  scripts/scripts.test.mjs              # MODIFY: tests for the blog/cast/author frontmatter conversion
  src/content/blog/*.md                 # CREATE (generated): 5 posts
  src/content/casts/*.md                # CREATE (generated): 7 casts
  src/content/authors/*.md              # CREATE (generated): 2 authors
  src/components/
    PostCard.astro                      # CREATE: a blog post summary card (warm postcard style)
    CastCard.astro                      # CREATE: a screencast card (YouTube thumbnail + meta)
    ProseArticle.astro                  # CREATE: article body wrapper (warm typography + warm-dark code)
  src/pages/
    blog/index.astro                    # CREATE: /blog listing
    blog/[slug].astro                   # CREATE: /blog/<slug>
    casts/index.astro                   # CREATE: /casts grid
    casts/[slug].astro                  # CREATE: /casts/<slug> (embed + notes)
    authors/[slug].astro                # CREATE: /authors/<slug> (bio + their posts)
    blog/rss.xml.ts                     # CREATE: RSS feed
    blog/atom.xml.ts                    # CREATE: Atom feed (or alias to rss)
  astro.config.mjs                      # MODIFY only if a redirect is needed for parity
```

---

### Task 1: Content collections, schemas, and frontmatter migration

**Files:** `website/src/content.config.ts` (modify), `website/scripts/migrate-blog-casts.mjs` (create), `website/scripts/scripts.test.mjs` (modify), generated content under `website/src/content/{blog,casts,authors}/`.

**Interfaces:**
- Produces: `convertBlogFrontmatter(raw, kind)` where `kind ∈ {'blog','cast','author'}` — pure Zola-TOML→YAML transform per the mapping. Collections `blog`, `casts`, `authors` with Zod schemas: blog `{title, description, pubDate: z.date(), updatedDate: z.date().optional(), authors: z.array(z.string())}`; casts `{title, description, pubDate, authors, episode: z.string(), youtube: z.string()}`; authors `{name, description}`.

- [ ] **Step 1: Failing tests** in `scripts.test.mjs` for `convertBlogFrontmatter`:

```js
import { convertBlogFrontmatter } from './migrate-blog-casts.mjs';
it('blog: maps date→pubDate, taxonomy authors→slug array, drops template', () => {
  const z = `+++\ntitle = "Hello"\ndescription = "d"\ndate = 2024-01-25T18:03:52+01:00\ndraft = false\ntemplate = "blog/page.html"\n[taxonomies]\nauthors = ["Team Loco"]\n+++\n\nbody\n`;
  const out = convertBlogFrontmatter(z, 'blog');
  expect(out).toContain('title: Hello');
  expect(out).toContain('pubDate: 2024-01-25');
  expect(out).toContain('authors:\n  - team-loco');
  expect(out).not.toContain('template');
  expect(out).toContain('\nbody\n');
});
it('cast: maps extra.num→episode and extra.id→youtube', () => {
  const z = `+++\ntitle = "T"\ndescription = "d"\ndate = 2024-06-27T14:20:42+00:00\ntemplate = "casts/page.html"\n[taxonomies]\nauthors = ["Team Loco"]\n[extra]\nnum = "007"\nid = "OWUvUSC1KvY"\n+++\nnotes\n`;
  const out = convertBlogFrontmatter(z, 'cast');
  expect(out).toContain('episode: "007"');
  expect(out).toContain('youtube: OWUvUSC1KvY');
});
```

- [ ] **Step 2: Run** `cd website && pnpm test` — FAIL (no module).
- [ ] **Step 3: Implement `migrate-blog-casts.mjs`** — parse TOML via `smol-toml`; per `kind`, emit YAML with the mapped keys (dates as ISO `YYYY-MM-DD`; author slugs = `value.toLowerCase().replace(/\s+/g,'-')`); preserve body verbatim; skip files with `draft = true`. Add a CLI walker that writes `blog/`, `casts/`, `authors/` into `website/src/content/` (drop `_index.md`).
- [ ] **Step 4: Run** — PASS.
- [ ] **Step 5: Add the collections** to `website/src/content.config.ts` with the Zod schemas above (glob loaders for the three dirs).
- [ ] **Step 6: Migrate + build** — `node scripts/migrate-blog-casts.mjs && pnpm build`. Expected: 5 blog + 7 casts + 2 authors present; build succeeds (pages come in later tasks — this task only needs the collections to type-check and build).
- [ ] **Step 7: Commit** — `git add website/src/content.config.ts website/scripts website/src/content/blog website/src/content/casts website/src/content/authors && git commit -m "feat(blog): content collections + migrate blog/casts/authors frontmatter"`

---

### Task 2: Blog index + post pages + author byline

**Files:** `website/src/pages/blog/index.astro`, `website/src/pages/blog/[slug].astro`, `website/src/components/PostCard.astro`, `website/src/components/ProseArticle.astro`.

**Interfaces:**
- Consumes: `blog`/`authors` collections (Task 1), `Base`/`Nav`/`Footer`, tokens.
- Produces: `/blog/` (list) and `/blog/<slug>/` (article). `PostCard.astro` (props: post entry) — a warm postcard-style summary (title, description, date, author name). `ProseArticle.astro` — a slot wrapper styling rendered markdown (headings, paragraphs, lists, links in `--red-ink`, and warm-dark `<pre>` code blocks matching the docs/homepage).

- [ ] **Step 1:** `PostCard.astro` — warm card (paper/card bg, dashed-frame optional, title in display type, description, `date · author`), links to `/blog/<slug>/`. Reuse the postcard aesthetic from the homepage pillars (subtle, tasteful — this is editorial).
- [ ] **Step 2:** `ProseArticle.astro` — scoped styles for the article body: comfortable measure (~68ch), warm typography, `h2/h3` scale, inline code in `--paper-2`/`--red-ink`, and `pre` blocks in the warm-dark palette (`#1b1512` etc., same as `CodeWindow`). Both themes.
- [ ] **Step 3:** `blog/index.astro` — `getCollection('blog')`, sort by `pubDate` desc, render a header ("Blog") + a list/grid of `PostCard`s. Under `Base` + `Nav`/`Footer`.
- [ ] **Step 4:** `blog/[slug].astro` — `getStaticPaths` over `blog`; render title, meta row (date + author link to `/authors/<slug>/`), then the post body via `render(entry)` wrapped in `ProseArticle`. `trailingSlash` parity.
- [ ] **Step 5: Verify** — `pnpm build` + headless-Chrome screenshots of `/blog/` and one post in light+dark: warm identity, readable article, code blocks warm-dark, author link present. Confirm all 5 posts build at `/blog/<slug>/`.
- [ ] **Step 6: Commit** — `git add website/src/pages/blog website/src/components/PostCard.astro website/src/components/ProseArticle.astro && git commit -m "feat(blog): blog index + post article pages in the warm identity"`

---

### Task 3: Casts index (grid) + cast pages (embed + notes)

**Files:** `website/src/pages/casts/index.astro`, `website/src/pages/casts/[slug].astro`, `website/src/components/CastCard.astro`.

**Interfaces:**
- Consumes: `casts` collection, `Base`/`Nav`/`Footer`, `ProseArticle` (Task 2), tokens.
- Produces: `/casts/` (grid) + `/casts/<slug>/` (embed + notes). `CastCard.astro` — a card showing the YouTube thumbnail (`https://img.youtube.com/vi/<youtube>/hqdefault.jpg`), episode number badge, title, description; links to `/casts/<slug>/`.

- [ ] **Step 1:** `CastCard.astro` — warm card with the YouTube thumbnail (lazy `<img>`), an episode badge (`#007`), title, description. Hover lift.
- [ ] **Step 2:** `casts/index.astro` — `getCollection('casts')`, sort by `episode` asc (or `pubDate`), render a header ("Screencasts") + a responsive grid of `CastCard`s.
- [ ] **Step 3:** `casts/[slug].astro` — `getStaticPaths` over `casts`; render title, meta, a responsive YouTube embed (`<iframe>` `https://www.youtube-nocookie.com/embed/<youtube>` in a 16:9 wrapper, `loading="lazy"`, `title` set), then the notes body via `render(entry)` in `ProseArticle`.
- [ ] **Step 4: Verify** — `pnpm build` + screenshots of `/casts/` and one cast in light+dark: grid with thumbnails, working embed markup, notes styled. All 7 casts build.
- [ ] **Step 5: Commit** — `git add website/src/pages/casts website/src/components/CastCard.astro && git commit -m "feat(casts): screencasts grid + cast pages with YouTube embed"`

---

### Task 4: Author pages

**Files:** `website/src/pages/authors/[slug].astro`.

**Interfaces:**
- Consumes: `authors`/`blog` collections, `Base`/`Nav`/`Footer`, `ProseArticle`.
- Produces: `/authors/<slug>/` — author name, bio (rendered body), and the list of that author's blog posts (`PostCard`s where `post.data.authors` includes the slug).

- [ ] **Step 1:** `authors/[slug].astro` — `getStaticPaths` over `authors`; render name + bio (via `render`); then `getCollection('blog')` filtered to posts whose `authors` includes this slug, shown as `PostCard`s (or a simple list if none).
- [ ] **Step 2: Verify** — `pnpm build`; both author pages build at `/authors/<slug>/`; each lists the right posts; screenshot one. The blog/post author byline links here (from Task 2) — confirm the link resolves.
- [ ] **Step 3: Commit** — `git add website/src/pages/authors && git commit -m "feat(blog): author pages listing their posts"`

---

### Task 5: RSS/Atom feeds + URL parity + a11y

**Files:** `website/src/pages/blog/rss.xml.ts`, `website/src/pages/blog/atom.xml.ts`, `website/scripts/url-parity-blog.mjs`, `website/package.json` (add `@astrojs/rss`).

**Interfaces:**
- Consumes: `blog` collection.

- [ ] **Step 1: RSS** — `pnpm --dir website add @astrojs/rss`; `blog/rss.xml.ts` uses `@astrojs/rss` with site title, description, and `getCollection('blog')` items (title, description, `pubDate`, `link: /blog/<slug>/`). `blog/atom.xml.ts` — emit an Atom-format feed of the same items (either `@astrojs/rss` if it supports atom, or a small hand-built Atom XML). Both must be valid XML at their URLs.
- [ ] **Step 2: Verify feeds** — `pnpm build`; `dist/blog/rss.xml` and `dist/blog/atom.xml` exist, are valid XML, and list all 5 posts with correct `/blog/<slug>/` links.
- [ ] **Step 3: URL parity** — `url-parity-blog.mjs`: enumerate OLD URLs from `docs-site/content/{blog,casts,authors}/**` (`/blog/<slug>/`, `/casts/<slug>/`, `/authors/<slug>/`, plus `/blog/`, `/casts/`, and the two feed URLs) and NEW from `website/dist/**`; print any missing. Add redirects in `astro.config.mjs` if a slug differs. Expected: 0 missing.
- [ ] **Step 4: a11y/perf** — build; confirm the blog/casts/author pages have proper landmarks/headings; the YouTube embeds are `loading="lazy"` with `title`; images have `alt`. Spot-check one page with a headless-Chrome a11y snapshot.
- [ ] **Step 5: Commit** — `git add website/src/pages/blog/rss.xml.ts website/src/pages/blog/atom.xml.ts website/scripts/url-parity-blog.mjs website/package.json website/pnpm-lock.yaml astro.config.mjs && git commit -m "feat(blog): RSS+Atom feeds, URL parity, a11y pass"`

---

## Follow-on (out of scope)

- **Cutover** — switch the deploy from `docs-site/` to `website/`, delete `docs-site/` — its own plan (after this).

## Self-Review

- **Spec coverage:** collections+migration → Task 1; blog list/post → Task 2; casts grid/page → Task 3; authors → Task 4; feeds+parity+a11y → Task 5. The spec's "Blog & Screencasts" requirements (content collections, author pages, RSS at existing feed URLs, restyled shells) are all assigned.
- **Placeholder scan:** the converter + feeds have concrete mappings/tests; no vague steps.
- **Type/name consistency:** `convertBlogFrontmatter(raw, kind)` is the pure transform (Task 1, used nowhere else). Collection names `blog`/`casts`/`authors` and the schema field names (`pubDate`, `authors`, `episode`, `youtube`, `name`) are defined in Task 1 and consumed by Tasks 2–5. Author slug derivation (lowercase + spaces→hyphens) is stated once and used by Tasks 1/2/4. `ProseArticle`/`PostCard`/`CastCard` are defined in Tasks 2/3 and reused in 3/4.
