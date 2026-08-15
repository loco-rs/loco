const REPOSITORY_API = 'https://api.github.com/repos/loco-rs/loco';
const CACHE_KEY = 'loco-github-stars';
const CACHE_REFRESH_INTERVAL_MS = 60 * 60 * 1000;

interface CachedStars {
  count: number;
  fetchedAt: number;
}

function isStarCount(value: unknown): value is number {
  return typeof value === 'number' && Number.isInteger(value) && value >= 0;
}

function formatStarCount(count: number): string {
  return new Intl.NumberFormat('en-US', {
    notation: count >= 1_000 ? 'compact' : 'standard',
    maximumFractionDigits: 1,
  })
    .format(count)
    .toLowerCase();
}

function updateGitHubStars(root: ParentNode, count: number): void {
  const compactCount = formatStarCount(count);

  root.querySelectorAll<HTMLElement>('[data-github-stars]').forEach((target) => {
    const label = target.querySelector<HTMLElement>('[data-star-label]');
    if (!label) return;

    label.textContent =
      target.dataset.starPlacement === 'after' ? `${compactCount} ★` : `★ ${compactCount}`;
    target.setAttribute('aria-label', `Loco on GitHub: ${count.toLocaleString('en-US')} stars`);
  });
}

function readCachedStars(storage: Storage | undefined): CachedStars | undefined {
  if (!storage) return undefined;

  try {
    const cached = JSON.parse(storage.getItem(CACHE_KEY) ?? 'null') as Partial<CachedStars> | null;
    if (cached && isStarCount(cached.count) && typeof cached.fetchedAt === 'number') {
      return { count: cached.count, fetchedAt: cached.fetchedAt };
    }
  } catch {
    // Storage can be unavailable in privacy modes; the API fallback still works.
  }

  return undefined;
}

function writeCachedStars(storage: Storage | undefined, count: number): void {
  if (!storage) return;

  try {
    storage.setItem(CACHE_KEY, JSON.stringify({ count, fetchedAt: Date.now() }));
  } catch {
    // A failed cache write should never stop the star count from rendering.
  }
}

export async function initGitHubStars(doc: Document): Promise<void> {
  let storage: Storage | undefined;
  try {
    storage = doc.defaultView?.localStorage;
  } catch {
    // Accessing localStorage itself can throw for opaque or sandboxed origins.
  }

  const cached = readCachedStars(storage);
  if (cached) {
    // Always render the last successful result, no matter how old it is. Its
    // age only controls when GitHub is queried again.
    updateGitHubStars(doc, cached.count);
    if (Date.now() - cached.fetchedAt < CACHE_REFRESH_INTERVAL_MS) return;
  }

  try {
    const response = await fetch(REPOSITORY_API, {
      headers: { Accept: 'application/vnd.github+json' },
      cache: 'no-store',
    });
    if (!response.ok) return;

    const repository = (await response.json()) as { stargazers_count?: unknown };
    if (!isStarCount(repository.stargazers_count)) return;

    updateGitHubStars(doc, repository.stargazers_count);
    writeCachedStars(storage, repository.stargazers_count);
  } catch {
    // Keep the stored count after a failed refresh. Only a first-time visitor
    // with no stored result sees the useful, non-numeric fallback.
  }
}
