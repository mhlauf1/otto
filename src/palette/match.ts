/**
 * Small fuzzy matcher for the command palette. Higher scores rank first; null means no match.
 * A prefix beats a word start, which beats a scattered subsequence.
 */
export function score(query: string, text: string): number | null {
  const q = query.trim().toLowerCase();
  if (!q) return 0;
  const t = text.toLowerCase();
  if (t.startsWith(q)) return 300 - t.length;
  const wordStart = t.indexOf(` ${q}`);
  if (wordStart >= 0) return 200 - wordStart;
  const contains = t.indexOf(q);
  if (contains >= 0) return 150 - contains;
  let ti = 0;
  let gaps = 0;
  for (const ch of q) {
    const found = t.indexOf(ch, ti);
    if (found < 0) return null;
    gaps += found - ti;
    ti = found + 1;
  }
  return 100 - gaps;
}
