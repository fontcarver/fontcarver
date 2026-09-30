import { call, type Command, type DocId, type GlyphId } from '../api'
import { emptyFilter } from '../stores/documentStore'

/** One persistent glyph name change. */
export interface GlyphRename {
  readonly from: string
  readonly to: string
}

/** A glyph name that must be corrected before applying a batch. */
export interface RenameIssue {
  readonly kind: 'invalid' | 'duplicate' | 'occupied' | 'missing'
  readonly name: string
}

/** Validated name changes and their single atomic document command. */
export interface RenamePlan {
  readonly renames: readonly GlyphRename[]
  readonly issues: readonly RenameIssue[]
  readonly command: Command | null
}

/** How the search text is interpreted. */
export interface ReplaceOptions {
  /** Treat the search as a JavaScript regular expression with capture groups. */
  readonly regex?: boolean
  /** Ignore case while matching. */
  readonly ignoreCase?: boolean
}

/** A search the user has not finished typing, reported instead of thrown. */
export class InvalidSearchError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'InvalidSearchError'
  }
}

/**
 * Find/replace over glyph names.
 *
 * Literal by default and case-sensitive, which is how glyph names compare. In
 * regular-expression mode the pattern replaces every match and `$1` refers to a
 * capture group, exactly as `String.replaceAll` defines it; a pattern that does
 * not compile raises {@link InvalidSearchError} so the dialog can say what is
 * wrong instead of leaving the preview empty. An empty search never changes a
 * name, so the dialog opens with nothing staged.
 */
export function replaceGlyphNames(
  names: readonly string[],
  find: string,
  replacement: string,
  options: ReplaceOptions = {},
): GlyphRename[] {
  if (!find) return []
  if (!options.regex && !options.ignoreCase) {
    return names.flatMap((from) => {
      const to = from.split(find).join(replacement)
      return from === to ? [] : [{ from, to }]
    })
  }
  const flags = options.ignoreCase ? 'giu' : 'gu'
  let pattern: RegExp
  try {
    pattern = new RegExp(options.regex ? find : escapeRegExp(find), flags)
  } catch (error) {
    throw new InvalidSearchError(
      error instanceof Error ? error.message : String(error),
    )
  }
  return names.flatMap((from) => {
    // A fresh index per name: the pattern is global and stateful.
    pattern.lastIndex = 0
    const to = from.replaceAll(pattern, replacement)
    return from === to ? [] : [{ from, to }]
  })
}

function escapeRegExp(text: string): string {
  return text.replaceAll(/[.*+?^${}()|[\]\\]/gu, '\\$&')
}

/**
 * Plans simultaneous renames using the existing atomic Batch API. Moving every
 * source aside first supports chains and cycles without replacing a glyph.
 * The kernel remains authoritative for collisions and unresolved references.
 */
export function planBatchRename(
  allNames: readonly string[],
  changes: readonly GlyphRename[],
  temporaryPrefix = '_fcRename',
): RenamePlan {
  const renames = changes.filter(({ from, to }) => from !== to)
  const occupied = new Set(allNames)
  const sources = new Set(renames.map(({ from }) => from))
  const targets = new Set<string>()
  const issues: RenameIssue[] = []
  for (const { from, to } of renames) {
    if (!occupied.has(from)) issues.push({ kind: 'missing', name: from })
    if (!/^[A-Za-z_.][A-Za-z0-9_.]*$/.test(to)) {
      issues.push({ kind: 'invalid', name: to })
    } else if (targets.has(to)) {
      issues.push({ kind: 'duplicate', name: to })
    } else if (occupied.has(to) && !sources.has(to)) {
      issues.push({ kind: 'occupied', name: to })
    }
    targets.add(to)
  }
  if (issues.length || !renames.length)
    return { renames, issues, command: null }
  const reserved = new Set([...occupied, ...targets])
  let serial = 0
  const temporary = renames.map(() => {
    let name: string
    do {
      name = `${temporaryPrefix}${serial++}`
    } while (reserved.has(name))
    reserved.add(name)
    return name
  })
  const commands: Command[] = []
  renames.forEach(({ from }, index) => {
    commands.push({ RenameGlyph: { from, to: temporary[index]! } })
  })
  renames.forEach(({ to }, index) => {
    commands.push({ RenameGlyph: { from: temporary[index]!, to } })
  })
  return { renames, issues, command: { Batch: commands } }
}

/** Names and stable handles needed for collision checking and preview. */
export interface RenameInventory {
  readonly gid: GlyphId
  readonly name: string
}

/** Reads the full inventory without outlines, including destinations offscreen. */
export async function loadRenameInventory(
  doc: DocId,
): Promise<RenameInventory[]> {
  const inventory: RenameInventory[] = []
  for (;;) {
    const page = await call('glyphs.list', {
      doc,
      master: null,
      filter: emptyFilter(),
      sort: 'Order',
      offset: inventory.length,
      limit: 2000,
      include_paths: false,
    })
    inventory.push(...page.items.map(({ gid, name }) => ({ gid, name })))
    if (!page.items.length || inventory.length >= page.total) return inventory
  }
}
