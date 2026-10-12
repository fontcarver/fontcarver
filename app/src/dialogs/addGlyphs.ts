import {
  call,
  type AddGlyphsResult,
  type DocId,
  type GeneratedGlyph,
} from '../api'

import { STARTER_RECIPES } from './newFont'

/** How long the recipe field waits before asking the kernel for a preview. */
export const PREVIEW_DEBOUNCE_MS = 200

/** One entry of the character-set picker. */
export interface CharsetOption {
  /** Key of its `dialog.addGlyphs.charset.*` label. */
  readonly id: string
  /** Recipe handed to `glyphs.add`. */
  readonly recipe: string
}

/**
 * Repertoires the picker offers, cheapest first: their counts are fetched in
 * this order so the small Latin sets appear while a large CJK set is still
 * being resolved. Any other repertoire the kernel knows is still reachable by
 * typing its `@charset:` item into the recipe field.
 */
export const CHARSETS: readonly CharsetOption[] = [
  { id: 'basicLatin', recipe: STARTER_RECIPES.basicLatin },
  { id: 'latinExtA', recipe: 'U+0100-U+017F' },
  { id: 'latin1', recipe: STARTER_RECIPES.latin1 },
  { id: 'gb2312', recipe: STARTER_RECIPES.gb2312 },
  { id: 'big5', recipe: '@charset:big5' },
  { id: 'jisx0208', recipe: '@charset:jisx0208' },
  { id: 'ksx1001', recipe: '@charset:ksx1001' },
  { id: 'iicore', recipe: '@charset:iicore' },
  { id: 'gb18030level1', recipe: '@charset:gb18030level1' },
]

/** Why the kernel skipped one item of a recipe. */
export type SkipReason = 'present' | 'unknown' | 'tooLarge'

/** Order the reasons are reported in: the common one first. */
const SKIP_ORDER: readonly SkipReason[] = ['present', 'unknown', 'tooLarge']

const TOO_LARGE_PREFIX = 'too-large: '

/** The kernel's own glyph-name grammar, which decides `Unknown` from a duplicate. */
const GLYPH_NAME = /^[A-Za-z_.][A-Za-z0-9_.]*$/

/**
 * Classifies one entry of `AddGlyphsResult.skipped`.
 *
 * The kernel prefixes only the oversized ranges; the other two reasons are told
 * apart by the same rule it used itself. Every item it planned to create has a
 * valid glyph name, so a bare name in the skip list is a glyph that already
 * exists, and anything else is a token the recipe grammar could not read.
 */
export function classifySkip(entry: string): SkipReason {
  if (entry.startsWith(TOO_LARGE_PREFIX)) return 'tooLarge'
  return GLYPH_NAME.test(entry) ? 'present' : 'unknown'
}

/** One reason and the skipped items that carry it. */
export interface SkipGroup {
  readonly reason: SkipReason
  readonly items: readonly string[]
}

/** Groups a skip list by reason, dropping reasons nothing was skipped for. */
export function groupSkips(skipped: readonly string[]): SkipGroup[] {
  return SKIP_ORDER.map((reason) => ({
    reason,
    items: skipped.filter((entry) => classifySkip(entry) === reason),
  })).filter((group) => group.items.length > 0)
}

/**
 * Resolves a recipe without applying it.
 *
 * The kernel owns the recipe grammar, the duplicate check and the expansion
 * limits, so the dialog's preview and its counts come from a dry run rather
 * than from a second parser in the interface.
 */
export function previewRecipe(
  doc: DocId,
  recipe: string,
  compose = false,
): Promise<RecipeOutcome> {
  return run(doc, recipe, compose, true)
}

/**
 * Counts what a recipe would create, without the cells.
 *
 * The character-set rows show one number each, and a legacy CJK repertoire
 * resolves tens of thousands of glyphs: building and sending a cell for every
 * one of them costs far more than the number on screen.
 */
export async function countRecipe(
  doc: DocId,
  recipe: string,
): Promise<{ added: number; skipped: readonly string[] }> {
  const result = await call('glyphdata.generate_glyphs', {
    doc,
    recipe,
    master: null,
    dry_run: true,
    count_only: true,
  })
  return { added: result.added_count ?? 0, skipped: result.skipped }
}

/** Creates the glyphs a recipe asks for. */
export function applyRecipe(
  doc: DocId,
  recipe: string,
  compose = false,
): Promise<RecipeOutcome> {
  return run(doc, recipe, compose, false)
}

/**
 * What a recipe did, whether or not compositions were built.
 *
 * `glyphs.add` creates blank glyphs; `glyphdata.generate_glyphs` creates the
 * same set but builds a composite wherever the glyph database defines one, and
 * reports per glyph which components it used and which it could not find. The
 * dialog shows the same preview either way, so the two share one shape.
 */
export interface RecipeOutcome extends AddGlyphsResult {
  /** One entry per created glyph when compositions were built. */
  readonly composed: readonly GeneratedGlyph[]
}

async function run(
  doc: DocId,
  recipe: string,
  compose: boolean,
  dryRun: boolean,
): Promise<RecipeOutcome> {
  if (compose) {
    return call('glyphdata.generate_glyphs', {
      doc,
      recipe,
      master: null,
      dry_run: dryRun,
      count_only: false,
    })
  }
  const result = await call('glyphs.add', {
    doc,
    recipe,
    master: null,
    dry_run: dryRun,
  })
  return { ...result, composed: [] }
}
