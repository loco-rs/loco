import { readFileSync, writeFileSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// Zola's docs section used absolute-from-content-root links of the form
// `@/docs/<path>.md` (optionally with a `#anchor`), resolved by Zola's link
// checker at build time. Starlight has no equivalent shorthand, so these are
// rewritten to plain site-relative URLs: drop the `.md` extension, prepend
// a leading slash, and keep any anchor untouched. Every other link form
// (external, relative `./x.md`, bare `#frag`, images, non-docs `@/` links)
// is left exactly as-is.
//
// Two surface forms occur in the migrated content: standard Markdown link
// syntax `](@/docs/<path>.md)`, and a handful of raw inline HTML anchors
// `<a href="@/docs/<path>.md">` (Markdown passes raw HTML through
// untouched, so these survive migration as literal `@/` links too).
const MARKDOWN_DOCS_LINK_RE = /\]\(@\/docs\/([^)#]+?)\.md(#[^)]*)?\)/g;
const HTML_HREF_DOCS_LINK_RE = /href="@\/docs\/([^"#]+?)\.md(#[^"]*)?"/g;

/**
 * Rewrites Zola-style `@/docs/<path>.md[#anchor]` links (in both Markdown
 * link syntax and raw HTML `href="..."` attributes) into Starlight-style
 * `/docs/<path>[#anchor]` links. All other link syntax is left untouched.
 *
 * @param {string} markdown
 * @returns {string}
 */
export function rewriteLinks(markdown) {
  return markdown
    .replace(MARKDOWN_DOCS_LINK_RE, (_match, docPath, anchor = '') => `](/docs/${docPath}${anchor})`)
    .replace(HTML_HREF_DOCS_LINK_RE, (_match, docPath, anchor = '') => `href="/docs/${docPath}${anchor}"`);
}

const DOCS_ROOT = path.resolve(__dirname, '../src/content/docs/docs');

function walk(dir) {
  const files = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...walk(full));
    } else if (entry.name.endsWith('.md')) {
      files.push(full);
    }
  }
  return files;
}

function countMatches(text) {
  const markdownCount = text.match(MARKDOWN_DOCS_LINK_RE)?.length ?? 0;
  const htmlCount = text.match(HTML_HREF_DOCS_LINK_RE)?.length ?? 0;
  return markdownCount + htmlCount;
}

function apply() {
  const files = walk(DOCS_ROOT);
  let changedFiles = 0;
  let totalLinks = 0;

  for (const file of files) {
    const raw = readFileSync(file, 'utf8');
    const linkCount = countMatches(raw);
    if (linkCount === 0) continue;

    const rewritten = rewriteLinks(raw);
    if (rewritten !== raw) {
      writeFileSync(file, rewritten, 'utf8');
      changedFiles += 1;
      totalLinks += linkCount;
    }
  }

  console.log(`rewrote ${totalLinks} link(s) across ${changedFiles} file(s)`);
}

if (process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) {
  if (process.argv.includes('--apply')) {
    apply();
  } else {
    console.log('Usage: node rewrite-links.mjs --apply');
    process.exit(1);
  }
}
