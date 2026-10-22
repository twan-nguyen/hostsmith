/**
 * Sublime Text–style fuzzy matching for the ⌘K palette. Rewards consecutive
 * matches, matches at the start of a word and exact case. Lengths count
 * characters (code points), not UTF-16 units.
 */
const WORD_CHAR = /[\p{L}\p{N}]/u;

/** The score of the subsequence match, or `null` when `query` is not a subsequence. */
export function fuzzyMatch(query: string, candidate: string): number | null {
  if (query === "") return 0;
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
  return score - (c.length - matched.length) * 0.05;
}
