import { readFileSync, readdirSync, mkdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// Content root mirrors the site's URL space 1:1: `src/content/docs/docs/**`
// serves `/docs/**` (see astro.config.mjs `trailingSlash: 'always'`).
const CONTENT_ROOT = path.resolve(__dirname, '../src/content/docs/docs');
const OUT_DIR = path.resolve(__dirname, '../public');
const SITE = 'https://loco.rs';

// Same six Diátaxis groups + labels as the sidebar in astro.config.mjs.
const SECTION_LABELS = {
  tutorials: 'Tutorials',
  'how-to': 'How-to guides',
  reference: 'Reference',
  explanation: 'Explanation',
  extras: 'Extras',
  resources: 'Resources',
};
const SECTION_ORDER = ['tutorials', 'how-to', 'reference', 'explanation', 'extras', 'resources'];

function walk(dir) {
  const entries = readdirSync(dir, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) files.push(...walk(full));
    else if (entry.name.endsWith('.md')) files.push(full);
  }
  return files;
}

function unquote(value) {
  const trimmed = value.trim();
  if (
    (trimmed.startsWith('"') && trimmed.endsWith('"')) ||
    (trimmed.startsWith("'") && trimmed.endsWith("'"))
  ) {
    try {
      return JSON.parse(trimmed.replace(/^'|'$/g, '"'));
    } catch {
      return trimmed.slice(1, -1);
    }
  }
  return trimmed;
}

/**
 * Minimal parser for the flat YAML frontmatter this site's docs actually
 * produce (see scripts/convert-frontmatter.mjs): a `---` delimited block of
 * `title:`, `description:`, and `sidebar:\n  order: N`.
 *
 * @param {string} raw
 * @returns {{ title: string, description: string, order: number, body: string }}
 */
export function parseDoc(raw) {
  const match = raw.match(/^---\r?\n([\s\S]*?)\r?\n---\r?\n?/);
  if (!match) return { title: '', description: '', order: 999, body: raw };

  const fm = match[1];
  const body = raw.slice(match[0].length).trim();

  const title = unquote(fm.match(/^title:\s*(.+)$/m)?.[1] ?? '');
  const description = unquote(fm.match(/^description:\s*(.*)$/m)?.[1] ?? '');
  const order = Number(fm.match(/^\s+order:\s*(\d+)/m)?.[1] ?? 999);

  return { title, description, order, body };
}

export function urlFor(relPath) {
  // relPath is relative to CONTENT_ROOT, e.g. 'index.md', 'how-to/index.md',
  // 'how-to/add-model.md'.
  const dir = path.dirname(relPath);
  const base = path.basename(relPath, '.md');
  if (base === 'index') {
    return dir === '.' ? '/docs/' : `/docs/${dir}/`;
  }
  return `/docs/${dir}/${base}/`;
}

function loadPages() {
  return walk(CONTENT_ROOT)
    .map((file) => {
      const relPath = path.relative(CONTENT_ROOT, file);
      const raw = readFileSync(file, 'utf8');
      const doc = parseDoc(raw);
      const section = path.dirname(relPath).split(path.sep)[0]; // '.' for docs/index.md
      return { ...doc, url: urlFor(relPath), section, relPath };
    })
    .sort((a, b) => a.order - b.order || a.url.localeCompare(b.url));
}

/**
 * Builds `llms.txt` per the llmstxt.org convention: an H1 title, a summary
 * blockquote, then one H2 per Diátaxis section listing every page as a
 * `[title](url): description` bullet.
 */
function buildLlmsTxt(pages) {
  const root = pages.find((p) => p.section === '.');
  const lines = ['# Loco', ''];
  lines.push(`> ${root?.description || 'Loco is a Rust web framework for full-stack productivity, batteries included.'}`);
  lines.push('');
  if (root) {
    lines.push(`- [${root.title}](${SITE}${root.url})${root.description ? `: ${root.description}` : ''}`);
    lines.push('');
  }

  for (const section of SECTION_ORDER) {
    const sectionPages = pages.filter((p) => p.section === section);
    if (sectionPages.length === 0) continue;
    lines.push(`## ${SECTION_LABELS[section]}`);
    for (const page of sectionPages) {
      const desc = page.description ? `: ${page.description}` : '';
      lines.push(`- [${page.title}](${SITE}${page.url})${desc}`);
    }
    lines.push('');
  }

  return lines.join('\n').trimEnd() + '\n';
}

/**
 * Builds `llms-full.txt`: the full rendered-markdown body of every doc page,
 * concatenated in the same section/order as `llms.txt`, each preceded by
 * its title and canonical URL so an LLM can attribute/cite a passage.
 */
function buildLlmsFullTxt(pages) {
  const ordered = [
    ...pages.filter((p) => p.section === '.'),
    ...SECTION_ORDER.flatMap((section) => pages.filter((p) => p.section === section)),
  ];

  return (
    ordered
      .map((page) => `# ${page.title}\n\nSource: ${SITE}${page.url}\n\n${page.body}`)
      .join('\n\n---\n\n') + '\n'
  );
}

function main() {
  const pages = loadPages();
  mkdirSync(OUT_DIR, { recursive: true });

  const llmsTxt = buildLlmsTxt(pages);
  const llmsFullTxt = buildLlmsFullTxt(pages);

  writeFileSync(path.join(OUT_DIR, 'llms.txt'), llmsTxt, 'utf8');
  writeFileSync(path.join(OUT_DIR, 'llms-full.txt'), llmsFullTxt, 'utf8');

  console.log(`llms.txt: ${llmsTxt.length} bytes, ${pages.length} pages linked`);
  console.log(`llms-full.txt: ${llmsFullTxt.length} bytes, ${pages.length} pages inlined`);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}
