// Estimated reading time (whole minutes) for a markdown body, at the
// conventional ~200 words/minute. A word is any run of non-whitespace, which
// slightly over-counts code fences — fine for a "N min read" signal, and it
// matches what most blogs display. Always at least 1 minute.
export function readingTimeMinutes(body: string): number {
  const words = body.trim().split(/\s+/).filter(Boolean).length;
  return Math.max(1, Math.round(words / 200));
}
