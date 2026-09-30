/** Canonical free-text tags: comma separated, trimmed and deduplicated. */
export function parseTags(input: string): string[] {
  return [
    ...new Set(
      input
        .split(',')
        .map((tag) => tag.trim())
        .filter(Boolean),
    ),
  ]
}

/** Matching document tags for the final comma-separated token. */
export function tagSuggestions(
  input: string,
  known: readonly string[],
): string[] {
  const parts = input.split(',')
  const partial = (parts.pop() ?? '').trim().toLocaleLowerCase()
  const used = new Set(parts.map((tag) => tag.trim()))
  return [...new Set(known)]
    .filter(
      (tag) => !used.has(tag) && tag.toLocaleLowerCase().includes(partial),
    )
    .sort((a, b) => a.localeCompare(b))
    .slice(0, 8)
}

/** Replaces the unfinished token with a selected existing tag. */
export function completeTag(input: string, suggestion: string): string[] {
  return parseTags([...input.split(',').slice(0, -1), suggestion].join(','))
}
