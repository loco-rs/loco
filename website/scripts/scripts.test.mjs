import { describe, it, expect } from 'vitest';
import { convertFrontmatter } from './convert-frontmatter.mjs';
import { rewriteLinks } from './rewrite-links.mjs';

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
