import { readdirSync, existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// The URLs the old Zola site published, frozen in `legacy-urls.json`.
//
// This used to walk `docs-site/content/docs` directly. That tree is gone —
// and it was never really a source of truth anyway: the set of URLs loco.rs
// once served is history, fixed at the moment of the migration, and cannot
// change again. Deriving it from a live tree meant deleting a page silently
// deleted the obligation to keep its URL working.
const LEGACY_URLS = path.resolve(__dirname, 'legacy-urls.json');
const NEW_DIST_DOCS_ROOT = path.resolve(__dirname, '../dist/docs');

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
 * Old Zola URL scheme: `/docs/<section>/<slug>/`, where a section (or the
 * docs root) index file is named `_index.md` and maps to `/docs/<section>/`
 * (or `/docs/` for the root), and every other `<slug>.md` maps to
 * `/docs/<section>/<slug>/`.
 *
 * @returns {string[]} old doc URLs, e.g. ['/docs/', '/docs/how-to/', '/docs/how-to/add-model/']
 */
export function oldDocUrls(root) {
  if (root === undefined) {
    return JSON.parse(readFileSync(LEGACY_URLS, 'utf8')).docs;
  }
  const files = walk(root, (name) => name.endsWith('.md'));
  return files.map((file) => {
    const rel = path.relative(root, file); // e.g. 'how-to/add-model.md', '_index.md', 'how-to/_index.md'
    const dir = path.dirname(rel); // '.' | 'how-to'
    const base = path.basename(rel, '.md'); // 'add-model' | '_index'

    if (base === '_index') {
      return dir === '.' ? '/docs/' : `/docs/${dir}/`;
    }
    return `/docs/${dir}/${base}/`;
  });
}

/**
 * New Starlight build URL scheme, read straight off the built `dist/docs`
 * tree (with `trailingSlash: 'always'`, every route is `<path>/index.html`).
 *
 * @returns {string[]} new doc URLs, e.g. ['/docs/', '/docs/how-to/', '/docs/how-to/add-model/']
 */
export function newDocUrls(root = NEW_DIST_DOCS_ROOT) {
  const files = walk(root, (name) => name === 'index.html');
  return files.map((file) => {
    const rel = path.relative(root, file); // e.g. 'how-to/add-model/index.html', 'index.html'
    const dir = path.dirname(rel); // '.' | 'how-to/add-model'
    return dir === '.' ? '/docs/' : `/docs/${dir}/`;
  });
}

function main() {
  if (!existsSync(NEW_DIST_DOCS_ROOT)) {
    console.error(
      `${NEW_DIST_DOCS_ROOT} does not exist — run \`pnpm build\` before \`node scripts/url-parity.mjs\`.`
    );
    process.exit(1);
  }

  const oldUrls = oldDocUrls();
  const newUrls = new Set(newDocUrls());

  const missing = oldUrls.filter((url) => !newUrls.has(url)).sort();

  if (missing.length === 0) {
    console.log(`0 missing (checked ${oldUrls.length} old doc URLs against ${newUrls.size} new doc URLs)`);
  } else {
    console.log(`${missing.length} missing:`);
    for (const url of missing) console.log(`  ${url}`);
    process.exitCode = 1;
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}
