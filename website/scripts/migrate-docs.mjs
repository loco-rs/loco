import { readFileSync, writeFileSync, mkdirSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { convertFrontmatter } from './convert-frontmatter.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// Source: the (read-only) Zola docs tree. Destination: the Starlight docs
// collection, mirroring the same relative paths, with each section's
// `_index.md` renamed to `index.md`.
const SRC_ROOT = path.resolve(__dirname, '../../docs-site/content/docs');
const DEST_ROOT = path.resolve(__dirname, '../src/content/docs/docs');

function walk(dir) {
  const entries = readdirSync(dir, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...walk(full));
    } else if (entry.name.endsWith('.md')) {
      files.push(full);
    }
  }
  return files;
}

function migrate() {
  const files = walk(SRC_ROOT);
  const counts = {};

  for (const srcFile of files) {
    const relPath = path.relative(SRC_ROOT, srcFile);
    const relDir = path.dirname(relPath); // '.' for the root _index.md
    const baseName = path.basename(relPath) === '_index.md' ? 'index.md' : path.basename(relPath);
    const destRelPath = path.join(relDir, baseName);
    const destFile = path.join(DEST_ROOT, destRelPath);

    const raw = readFileSync(srcFile, 'utf8');
    const converted = convertFrontmatter(raw);

    mkdirSync(path.dirname(destFile), { recursive: true });
    writeFileSync(destFile, converted, 'utf8');

    const section = relDir === '.' ? 'index' : relDir.split(path.sep)[0];
    counts[section] = (counts[section] ?? 0) + 1;
  }

  const sectionOrder = ['tutorials', 'how-to', 'reference', 'explanation', 'extras', 'resources', 'index'];
  let total = 0;
  for (const section of sectionOrder) {
    if (counts[section] === undefined) continue;
    console.log(`${section}: ${counts[section]}`);
    total += counts[section];
  }
  for (const section of Object.keys(counts)) {
    if (!sectionOrder.includes(section)) {
      console.log(`${section}: ${counts[section]}`);
      total += counts[section];
    }
  }
  console.log(`total: ${total}`);
}

migrate();
