import { call, type AddGlyphsResult, type DocId } from '../api'

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
]

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
): Promise<AddGlyphsResult> {
  return call('glyphs.add', { doc, recipe, master: null, dry_run: true })
}

/** Creates the glyphs a recipe asks for. */
export function applyRecipe(
  doc: DocId,
  recipe: string,
): Promise<AddGlyphsResult> {
  return call('glyphs.add', { doc, recipe, master: null, dry_run: false })
}
