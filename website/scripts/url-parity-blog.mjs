import { readdirSync, existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// The blog/casts/authors URLs the old Zola site published, frozen in
// `legacy-urls.json` alongside the doc URLs — see the note in
// `url-parity.mjs` for why these are data and not a tree walk.
const LEGACY_URLS = path.resolve(__dirname, 'legacy-urls.json');
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
 * All old URLs expected to survive the migration: per-entry blog/casts/
 * authors URLs, the two section index pages, and the two feed URLs that
 * lived at `/blog/rss.xml` and `/blog/atom.xml`.
 *
 * Note on authors: a post can name an author slug that never had its own
 * file (`deploy-aws.md`'s `antonio-souza`), so that slug never had a
 * `/authors/<slug>/` page and is correctly absent from this list.
 *
 * @returns {string[]}
 */
export function oldUrls() {
  return JSON.parse(readFileSync(LEGACY_URLS, 'utf8')).entries;
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
