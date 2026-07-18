import { parse } from 'smol-toml';

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

/**
 * Converts a leading Zola `+++ TOML +++` frontmatter block into a Starlight
 * `--- YAML ---` block, per the documented field mapping. The body (and the
 * rest of the file after the frontmatter) is left byte-for-byte untouched.
 * Files with no leading `+++` block are returned unchanged.
 *
 * @param {string} rawMarkdown
 * @returns {string}
 */
export function convertFrontmatter(rawMarkdown) {
  const match = rawMarkdown.match(FRONTMATTER_RE);
  if (!match) return rawMarkdown;

  const toml = parse(match[1]);
  const body = rawMarkdown.slice(match[0].length);

  const lines = ['---'];
  if ('title' in toml) {
    lines.push(`title: ${yamlScalar(String(toml.title))}`);
  }
  if ('description' in toml) {
    lines.push(`description: ${yamlScalar(String(toml.description))}`);
  }
  if ('weight' in toml) {
    lines.push('sidebar:', `  order: ${toml.weight}`);
  }
  lines.push('---');

  return lines.join('\n') + '\n' + body;
}
