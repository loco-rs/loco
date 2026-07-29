import { readdirSync, existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// The old Zola blog/casts/authors content (read-only) and the new Astro
// build output.
const OLD_CONTENT_ROOT = path.resolve(__dirname, '../../docs-site/content');
const NEW_DIST_ROOT = path.resolve(__dirname, '../dist');

function walk(dir, predicate) {
  const entries = readdirSync(dir, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...walk(full, predicate));
    } else if (predicate(entry.name)) {
      files.push(full);
    }
  }
  return files;
}

/**
 * Old Zola URL scheme for a `blog`/`casts`/`authors` section: every
 * `<slug>.md` maps to `/<section>/<slug>/`; `_index.md` (the section
 * listing page) is skipped — it maps to the static `/<section>/` URL added
 * separately in `oldUrls()`, not to a per-entry URL here.
 *
 * Author slugs are read straight off the files that exist under
 * `content/authors/` — a post can reference an author slug with no author
 * file at all (e.g. `deploy-aws.md`'s `antonio-souza`), and since that
 * slug never gets its own file there, it's naturally never enumerated or
 * expected to have a `/authors/<slug>/` page.
 *
 * @returns {string[]} e.g. ['/blog/hello-world/', '/casts/001-.../', '/authors/team-loco/']
 */
export function oldSectionUrls(section, root = path.join(OLD_CONTENT_ROOT, section)) {
  const files = walk(root, (name) => name.endsWith('.md') && name !== '_index.md');
  return files.map((file) => {
    const slug = path.basename(file, '.md');
    return `/${section}/${slug}/`;
  });
}

/**
 * All old URLs expected to survive the migration: per-entry blog/casts/
 * authors URLs, the two section index pages, and the two feed URLs that
 * lived at `/blog/rss.xml` and `/blog/atom.xml`.
 *
 * @returns {string[]}
 */
export function oldUrls() {
  return [
    ...oldSectionUrls('blog'),
    ...oldSectionUrls('casts'),
    ...oldSectionUrls('authors'),
    '/blog/',
    '/casts/',
    '/blog/rss.xml',
    '/blog/atom.xml',
  ];
}

/**
 * New build URLs, read straight off `website/dist/**`. An `index.html`
 * maps to its containing directory (with trailing slash, matching
 * `trailingSlash: 'always'`); any other file maps to its path as-is (e.g.
 * `dist/blog/rss.xml` -> `/blog/rss.xml`).
 *
 * @returns {string[]}
 */
export function newUrls(root = NEW_DIST_ROOT) {
  const files = walk(root, () => true);
  return files.map((file) => {
    const rel = path.relative(root, file);
    if (path.basename(rel) === 'index.html') {
      const dir = path.dirname(rel);
      return dir === '.' ? '/' : `/${dir}/`;
    }
    return `/${rel}`;
  });
}

function main() {
  if (!existsSync(NEW_DIST_ROOT)) {
    console.error(`${NEW_DIST_ROOT} does not exist — run \`pnpm build\` before \`node scripts/url-parity-blog.mjs\`.`);
    process.exit(1);
  }

  const old = oldUrls();
  const fresh = new Set(newUrls());

  const missing = old.filter((url) => !fresh.has(url)).sort();

  if (missing.length === 0) {
    console.log(`0 missing (checked ${old.length} old blog/casts/authors URLs against ${fresh.size} new URLs)`);
  } else {
    console.log(`${missing.length} missing:`);
    for (const url of missing) console.log(`  ${url}`);
    process.exitCode = 1;
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}
