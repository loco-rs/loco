/**
 * The marketing components were ported "verbatim from the reference mockup",
 * and the mockup's placeholders came along with them: both primary CTAs were
 * `href="#"`, the wordmark was a `<div>` so there was no way home from /blog
 * or /casts, the footer's "Docs · GitHub · Discord · Blog" was a single span
 * of text, and the star count was the literal string `6.9k` in three files
 * while the repo was at 9066. All of it shipped, and all of it was reported
 * from outside (#1794).
 *
 * The markup checks read the component sources rather than rendered output on
 * purpose: the defect is in the markup as authored, and a source-level check
 * needs no build step to run.
 */
import { readFileSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { JSDOM } from 'jsdom';
import { describe, it, expect, vi, afterEach } from 'vitest';

import { initGitHubStars } from '../lib/github-stars';

const srcDir = path.dirname(path.dirname(fileURLToPath(import.meta.url)));

function astroFilesUnder(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) return astroFilesUnder(full);
    return entry.name.endsWith('.astro') ? [full] : [];
  });
}

const read = (relative: string) => readFileSync(path.join(srcDir, relative), 'utf8');

it('no component or page ships a placeholder `href="#"`', () => {
  const offenders = [...astroFilesUnder(path.join(srcDir, 'components')), ...astroFilesUnder(path.join(srcDir, 'pages'))]
    .filter((file) => /href=("#"|'#')/.test(readFileSync(file, 'utf8')))
    .map((file) => path.relative(srcDir, file));

  expect(offenders).toEqual([]);
});

it('the wordmark links home, so /blog and /casts can get back', () => {
  // Starlight's own header already did this; the marketing nav did not, which
  // is exactly the asymmetry that got reported.
  for (const component of ['components/Nav.astro', 'components/starlight/Header.astro']) {
    expect(read(component)).toMatch(/<a[^>]*class="brand"[^>]*href="\/"/);
  }
});

it('the footer names are links, not text', () => {
  const footer = read('components/Footer.astro');
  for (const [label, href] of [
    ['Docs', '/docs/'],
    ['GitHub', 'https://github.com/loco-rs/loco'],
    ['Discord', 'https://discord.gg/fTvyBzwKS8'],
    ['Blog', '/blog/'],
  ]) {
    expect(footer).toContain(`href="${href}"`);
    expect(footer).toContain(`>${label}</a>`);
  }
});

it('nothing hardcodes a star count any more', () => {
  const offenders = astroFilesUnder(path.join(srcDir, 'components'))
    .filter((file) => /\d[\d.]*k\s*★|★\s*\d[\d.]*k/.test(readFileSync(file, 'utf8')))
    .map((file) => path.relative(srcDir, file));

  expect(offenders).toEqual([]);
});

it('every star slot carries the label element the updater writes into', () => {
  // `updateGitHubStars` bails on any `[data-github-stars]` without a
  // `[data-star-label]` child, so a slot missing one silently keeps its
  // placeholder forever rather than failing loudly.
  const offenders = astroFilesUnder(path.join(srcDir, 'components'))
    .filter((file) => {
      const source = readFileSync(file, 'utf8');
      return source.includes('data-github-stars') && !source.includes('data-star-label');
    })
    .map((file) => path.relative(srcDir, file));

  expect(offenders).toEqual([]);
});

describe('initGitHubStars', () => {
  const HOUR_MS = 60 * 60 * 1000;

  function page() {
    const dom = new JSDOM(
      `<a data-github-stars><span data-star-label>★ GitHub</span></a>
       <a data-github-stars data-star-placement="after"><b data-star-label>Stars</b> on GitHub</a>`,
      { url: 'https://loco.rs' },
    );
    dom.window.localStorage.clear();
    return dom;
  }

  const labels = (dom: JSDOM) =>
    [...dom.window.document.querySelectorAll('[data-star-label]')].map((el) => el.textContent);

  const respondWith = (count: number) =>
    vi.fn().mockResolvedValue({ ok: true, json: async () => ({ stargazers_count: count }) });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.useRealTimers();
  });

  it('renders the live count on both sides of the mark', async () => {
    vi.stubGlobal('fetch', respondWith(9066));
    const dom = page();

    await initGitHubStars(dom.window.document as unknown as Document);

    expect(labels(dom)).toEqual(['★ 9.1k', '9.1k ★']);
  });

  it('spells out the exact count for screen readers', async () => {
    vi.stubGlobal('fetch', respondWith(9066));
    const dom = page();

    await initGitHubStars(dom.window.document as unknown as Document);

    expect(dom.window.document.querySelector('[data-github-stars]')?.getAttribute('aria-label')).toBe(
      'Loco on GitHub: 9,066 stars',
    );
  });

  it('keeps the placeholder when GitHub is unreachable and nothing is cached', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('offline')));
    const dom = page();

    await initGitHubStars(dom.window.document as unknown as Document);

    expect(labels(dom)).toEqual(['★ GitHub', 'Stars']);
  });

  it('serves a stale count rather than a placeholder when the refresh fails', async () => {
    const dom = page();
    dom.window.localStorage.setItem(
      'loco-github-stars',
      JSON.stringify({ count: 9066, fetchedAt: Date.now() - 2 * HOUR_MS }),
    );
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('offline')));

    await initGitHubStars(dom.window.document as unknown as Document);

    expect(labels(dom)).toEqual(['★ 9.1k', '9.1k ★']);
  });

  it('does not call GitHub again while the cache is fresh', async () => {
    const dom = page();
    dom.window.localStorage.setItem(
      'loco-github-stars',
      JSON.stringify({ count: 9066, fetchedAt: Date.now() - HOUR_MS / 2 }),
    );
    const fetchSpy = respondWith(1);
    vi.stubGlobal('fetch', fetchSpy);

    await initGitHubStars(dom.window.document as unknown as Document);

    expect(fetchSpy).not.toHaveBeenCalled();
    expect(labels(dom)).toEqual(['★ 9.1k', '9.1k ★']);
  });

  it('refreshes once the cached count is older than an hour', async () => {
    const dom = page();
    dom.window.localStorage.setItem(
      'loco-github-stars',
      JSON.stringify({ count: 9066, fetchedAt: Date.now() - 2 * HOUR_MS }),
    );
    vi.stubGlobal('fetch', respondWith(9500));

    await initGitHubStars(dom.window.document as unknown as Document);

    expect(labels(dom)).toEqual(['★ 9.5k', '9.5k ★']);
  });

  it('ignores a corrupt cache entry instead of throwing', async () => {
    const dom = page();
    dom.window.localStorage.setItem('loco-github-stars', '{not json');
    vi.stubGlobal('fetch', respondWith(9066));

    await initGitHubStars(dom.window.document as unknown as Document);

    expect(labels(dom)).toEqual(['★ 9.1k', '9.1k ★']);
  });

  it('leaves the placeholder alone when the API answers with a non-count', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({ ok: true, json: async () => ({ stargazers_count: 'many' }) }),
    );
    const dom = page();

    await initGitHubStars(dom.window.document as unknown as Document);

    expect(labels(dom)).toEqual(['★ GitHub', 'Stars']);
    expect(dom.window.localStorage.getItem('loco-github-stars')).toBeNull();
  });

  it('leaves counts under a thousand unabbreviated', async () => {
    vi.stubGlobal('fetch', respondWith(999));
    const dom = page();

    await initGitHubStars(dom.window.document as unknown as Document);

    expect(labels(dom)).toEqual(['★ 999', '999 ★']);
  });
});
