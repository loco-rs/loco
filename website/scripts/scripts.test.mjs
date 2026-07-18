import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { describe, it, expect, afterEach } from 'vitest';
import { convertFrontmatter } from './convert-frontmatter.mjs';
import { rewriteLinks } from './rewrite-links.mjs';
import { oldDocUrls, newDocUrls } from './url-parity.mjs';
import { parseDoc, urlFor } from './generate-llms-txt.mjs';
import { convertBlogFrontmatter } from './migrate-blog-casts.mjs';

it('maps title/description/weight and drops zola-only keys', () => {
  const zola = `+++\ntitle = "Add a worker"\ndescription = "How to add a worker"\nsort_by = "weight"\nweight = 30\ntemplate = "docs/page.html"\n+++\n\n# Body\ntext {{ get_env(name='X') }}\n`;
  const out = convertFrontmatter(zola);
  expect(out).toMatch(/^---\n/);
  expect(out).toContain('title: Add a worker');
  expect(out).toContain('description: How to add a worker');
  expect(out).toContain('sidebar:\n  order: 30');
  expect(out).not.toContain('sort_by');
  expect(out).not.toContain('template');
  // body + literal config syntax preserved verbatim
  expect(out).toContain("text {{ get_env(name='X') }}");
});
it('quotes titles containing colons/special chars safely', () => {
  const zola = `+++\ntitle = "Loco: the tour"\nweight = 1\n+++\nbody\n`;
  expect(convertFrontmatter(zola)).toContain('title: "Loco: the tour"');
});

it('rewrites @/docs links, preserving anchors, dropping .md', () => {
  expect(rewriteLinks('see [cfg](@/docs/reference/configuration.md)')).toBe('see [cfg](/docs/reference/configuration.md)'.replace('.md', ''));
  expect(rewriteLinks('[m](@/docs/how-to/add-worker.md#queues)')).toBe('[m](/docs/how-to/add-worker#queues)');
});
it('leaves external and relative links alone', () => {
  const s = '[x](https://example.com) and [y](./local.md) and [z](#frag)';
  expect(rewriteLinks(s)).toBe(s);
});
it('rewrites @/docs links embedded in raw HTML href attributes too', () => {
  expect(rewriteLinks('<a href="@/docs/tutorials/saas-with-auth.md">x</a>')).toBe(
    '<a href="/docs/tutorials/saas-with-auth">x</a>'
  );
  expect(
    rewriteLinks('<a href="@/docs/reference/query-pagination.md#daterangebuilder">x</a>')
  ).toBe('<a href="/docs/reference/query-pagination#daterangebuilder">x</a>');
});

describe('url-parity', () => {
  let dir;
  afterEach(() => {
    if (dir) rmSync(dir, { recursive: true, force: true });
  });

  it('maps old Zola paths to /docs/<section>/<slug>/, with _index.md as the section root', () => {
    dir = mkdtempSync(path.join(tmpdir(), 'old-docs-'));
    writeFileSync(path.join(dir, '_index.md'), 'x');
    mkdirSync(path.join(dir, 'how-to'));
    writeFileSync(path.join(dir, 'how-to', '_index.md'), 'x');
    writeFileSync(path.join(dir, 'how-to', 'add-model.md'), 'x');

    expect(oldDocUrls(dir).sort()).toEqual(['/docs/', '/docs/how-to/', '/docs/how-to/add-model/']);
  });

  it('maps new dist/docs/**/index.html paths to the same /docs/.../ URL shape', () => {
    dir = mkdtempSync(path.join(tmpdir(), 'new-docs-'));
    writeFileSync(path.join(dir, 'index.html'), 'x');
    mkdirSync(path.join(dir, 'how-to'));
    writeFileSync(path.join(dir, 'how-to', 'index.html'), 'x');
    mkdirSync(path.join(dir, 'how-to', 'add-model'));
    writeFileSync(path.join(dir, 'how-to', 'add-model', 'index.html'), 'x');

    expect(newDocUrls(dir).sort()).toEqual(['/docs/', '/docs/how-to/', '/docs/how-to/add-model/']);
  });
});

describe('generate-llms-txt', () => {
  it('parses title/description/order out of the migrated YAML frontmatter', () => {
    const raw = '---\ntitle: Add a model\ndescription: Generate a model.\nsidebar:\n  order: 1\n---\n\nBody text.\n';
    expect(parseDoc(raw)).toEqual({
      title: 'Add a model',
      description: 'Generate a model.',
      order: 1,
      body: 'Body text.',
    });
  });

  it('tolerates an empty description', () => {
    const raw = '---\ntitle: Extras\ndescription: ""\nsidebar:\n  order: 5\n---\nBody.\n';
    const doc = parseDoc(raw);
    expect(doc.description).toBe('');
  });

  it('derives /docs/... URLs the same way for index and non-index pages', () => {
    expect(urlFor('index.md')).toBe('/docs/');
    expect(urlFor('how-to/index.md')).toBe('/docs/how-to/');
    expect(urlFor('how-to/add-model.md')).toBe('/docs/how-to/add-model/');
  });
});

describe('migrate-blog-casts', () => {
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
});
