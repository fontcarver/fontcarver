/**
 * A small subsequence matcher used by the command palette. It is deliberately
 * simple: a query matches when its characters appear in order in the candidate,
 * and the score rewards matches at word starts and runs of adjacent characters.
 */

/** Result of scoring one candidate against a query. */
export interface FuzzyMatch {
  /** Higher is better. */
  score: number
  /** Indexes of the matched characters in the candidate. */
  indexes: number[]
}

function isWordStart(text: string, index: number): boolean {
  if (index === 0) return true
  const previous = text[index - 1] ?? ''
  return /[\s._/\-·—]/.test(previous)
}

/**
 * Scores `candidate` against `query`, or returns `null` when the query is not a
 * subsequence of the candidate. An empty query matches everything with score 0.
 */
export function fuzzyMatch(
  query: string,
  candidate: string,
): FuzzyMatch | null {
  const needle = query.trim().toLowerCase()
  if (needle.length === 0) return { score: 0, indexes: [] }
  const haystack = candidate.toLowerCase()

  const indexes: number[] = []
  let score = 0
  let cursor = 0
  let previousIndex = -2

  for (const character of needle) {
    if (character === ' ') continue
    const found = haystack.indexOf(character, cursor)
    if (found < 0) return null
    indexes.push(found)
    score += 1
    if (found === previousIndex + 1) score += 4
    if (isWordStart(candidate, found)) score += 3
    previousIndex = found
    cursor = found + 1
  }

  // Prefer shorter candidates and matches that start early.
  score += Math.max(0, 12 - (indexes[0] ?? 0))
  score -= candidate.length / 100
  return { score, indexes }
}

/** Filters and ranks candidates, best match first. Ties keep the input order. */
export function fuzzyRank<T>(
  query: string,
  items: readonly T[],
  text: (item: T) => string,
): T[] {
  return items
    .map((item, index) => ({
      item,
      index,
      match: fuzzyMatch(query, text(item)),
    }))
    .filter((entry) => entry.match !== null)
    .sort(
      (a, b) =>
        (b.match?.score ?? 0) - (a.match?.score ?? 0) || a.index - b.index,
    )
    .map((entry) => entry.item)
}
