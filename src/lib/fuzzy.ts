/**
 * Sublime Text–style fuzzy matching for the ⌘K palette. Rewards consecutive
 * matches, matches at the start of a word and exact case. Offsets count
 * characters (code points), not UTF-16 units.
 */
export type FuzzyMatch = {
  score: number;
  /** Half-open `[start, end)` runs of matched characters, for highlighting. */
  ranges: [number, number][];
};

const WORD_CHAR = /[\p{L}\p{N}]/u;

/** The best subsequence match, or `null` when `query` is not a subsequence. */
export function fuzzyMatch(query: string, candidate: string): FuzzyMatch | null {
  if (query === "") return { score: 0, ranges: [] };
  const q = Array.from(query);
  const c = Array.from(candidate);
  if (c.length === 0) return null;

  const matched: number[] = [];
  let qi = 0;
  for (let ci = 0; qi < q.length && ci < c.length; ci++) {
    if (q[qi].toLowerCase() === c[ci].toLowerCase()) {
      matched.push(ci);
      qi++;
    }
  }
  if (qi < q.length) return null;

  let score = 0;
  let prev = -2;
  matched.forEach((ci, i) => {
    score += ci === prev + 1 ? 5 : 1;
    if (ci === 0 || !WORD_CHAR.test(c[ci - 1])) score += 4;
    if (q[i] === c[ci]) score += 1;
    prev = ci;
  });
  // Shorter candidates win a tie.
  score -= (c.length - matched.length) * 0.05;

  return { score, ranges: collapse(matched) };
}

function collapse(indices: number[]): [number, number][] {
  const ranges: [number, number][] = [];
  for (const i of indices) {
    const last = ranges[ranges.length - 1];
    if (last && last[1] === i) last[1] = i + 1;
    else ranges.push([i, i + 1]);
  }
  return ranges;
}
