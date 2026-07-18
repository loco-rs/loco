import { parse } from 'smol-toml';
import { readFileSync, writeFileSync, mkdirSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// Zola frontmatter is delimited by a `+++` line, then TOML, then another
// `+++` line. Only the first such block (right at the top of the file) is
// treated as frontmatter; anything else is body content.
const FRONTMATTER_RE = /^\+\+\+\r?\n([\s\S]*?)\r?\n\+\+\+\r?\n?/;

// Values that would be ambiguous or invalid as a plain (unquoted) YAML
// scalar: leading YAML indicator characters, empty strings, and strings
// that would otherwise parse as a different type (bool/null/number).
const LEADING_SPECIAL_RE = /^[-?:,[\]{}#&*!|>'"%@`]/;
const AMBIGUOUS_SCALAR_RE = /^(true|false|null|~|-?\d+(\.\d+)?)$/i;

function needsQuoting(value) {
  if (value === '') return true;
  if (LEADING_SPECIAL_RE.test(value)) return true;
  if (value.includes(':') || value.includes('#')) return true;
  if (/^\s|\s$/.test(value)) return true;
  if (AMBIGUOUS_SCALAR_RE.test(value)) return true;
  return false;
}

function yamlScalar(value) {
  return needsQuoting(value) ? JSON.stringify(value) : value;
}

function authorSlug(name) {
  return name.toLowerCase().replace(/\s+/g, '-');
}

// Formats a TOML date (Date object, or a string) as YAML-safe `YYYY-MM-DD`.
function isoDate(value) {
  const d = value instanceof Date ? value : new Date(value);
  return d.toISOString().slice(0, 10);
}

/**
 * Converts a leading Zola `+++ TOML +++` frontmatter block (blog post, cast,
 * or author page) into a content-collection `--- YAML ---` block, per the
 * documented field mapping for each `kind`. The body is left byte-for-byte
 * untouched. Returns `null` for posts marked `draft = true` (callers should
 * skip writing them). Files with no leading `+++` block are returned
 * unchanged.
 *
 * @param {string} rawMarkdown
 * @param {'blog'|'cast'|'author'} kind
 * @returns {string|null}
 */
export function convertBlogFrontmatter(rawMarkdown, kind) {
  const match = rawMarkdown.match(FRONTMATTER_RE);
  if (!match) return rawMarkdown;

  const toml = parse(match[1]);
  const body = rawMarkdown.slice(match[0].length);

  if (toml.draft === true) return null;

  const lines = ['---'];

  if (kind === 'author') {
    if ('title' in toml) lines.push(`name: ${yamlScalar(String(toml.title))}`);
    if ('description' in toml) lines.push(`description: ${yamlScalar(String(toml.description))}`);
    lines.push('---');
    return lines.join('\n') + '\n' + body;
  }

  // blog + cast share title/description/pubDate/updatedDate/authors
  if ('title' in toml) lines.push(`title: ${yamlScalar(String(toml.title))}`);
  if ('description' in toml) lines.push(`description: ${yamlScalar(String(toml.description))}`);
  if ('date' in toml) lines.push(`pubDate: ${isoDate(toml.date)}`);
  if ('updated' in toml) lines.push(`updatedDate: ${isoDate(toml.updated)}`);

  const authors = toml.taxonomies?.authors ?? [];
  if (authors.length) {
    lines.push('authors:');
    for (const name of authors) lines.push(`  - ${authorSlug(name)}`);
  }

  if (kind === 'cast') {
    if (toml.extra?.num !== undefined) lines.push(`episode: ${yamlScalar(String(toml.extra.num))}`);
    if (toml.extra?.id !== undefined) lines.push(`youtube: ${yamlScalar(String(toml.extra.id))}`);
  }

  lines.push('---');
  return lines.join('\n') + '\n' + body;
}

// --- CLI walker: migrates docs-site/content/{blog,casts,authors} into
// website/src/content/{blog,casts,authors}, dropping each section's
// `_index.md` (no equivalent needed in a content collection).

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const SRC_ROOT = path.resolve(__dirname, '../../docs-site/content');
const DEST_ROOT = path.resolve(__dirname, '../src/content');

const SECTIONS = [
  { dir: 'blog', kind: 'blog' },
  { dir: 'casts', kind: 'cast' },
  { dir: 'authors', kind: 'author' },
];

function migrateSection(dir, kind) {
  const srcDir = path.join(SRC_ROOT, dir);
  const destDir = path.join(DEST_ROOT, dir);
  mkdirSync(destDir, { recursive: true });

  let count = 0;
  for (const entry of readdirSync(srcDir, { withFileTypes: true })) {
    if (!entry.isFile() || !entry.name.endsWith('.md')) continue;
    if (entry.name === '_index.md') continue;

    const raw = readFileSync(path.join(srcDir, entry.name), 'utf8');
    const converted = convertBlogFrontmatter(raw, kind);
    if (converted === null) continue; // draft, skipped

    writeFileSync(path.join(destDir, entry.name), converted, 'utf8');
    count += 1;
  }
  return count;
}

function migrate() {
  let total = 0;
  for (const { dir, kind } of SECTIONS) {
    const count = migrateSection(dir, kind);
    console.log(`${dir}: ${count}`);
    total += count;
  }
  console.log(`total: ${total}`);
}

// Only run the CLI walker when this file is executed directly (not when
// imported for its `convertBlogFrontmatter` export by tests).
if (import.meta.url === `file://${process.argv[1]}`) {
  migrate();
}
