/**
 * Staged per-component transforms for the CJK transform grid.
 *
 * The grid edits a *draft* of each component's affine so a radical can be
 * positioned and scaled while watching a live preview, and the whole set is
 * then applied as one undo entry. Keeping the arithmetic here — what a field
 * writes, what the preview has to draw, and what counts as a usable transform —
 * is what makes the preview and the applied command provably the same edit.
 */

import type { Command, Component, ComponentView } from '../../../api'
import {
  composeAffine,
  invertAffine,
  isInvertible,
  type Affine,
} from '../../../editor/affine'
import type { LayerTarget } from '../../../editor/pointCommands'
import { setComponentCommand } from '../../../editor/pointCommands'

/**
 * The editable fields of one component, in the order the grid shows them.
 *
 * Position and scale come first because they are what placing a radical needs;
 * the two shear coefficients follow so an imported rotated or slanted component
 * is still fully editable rather than silently flattened.
 *
 * The keys are the kurbo coefficient names the `coefficients` strings already
 * use: `x`/`y` for the translation, `a`/`d` for the scales, `c`/`b` for shear.
 */
export const TRANSFORM_FIELDS = [
  { key: 'x', index: 4, step: 1 },
  { key: 'y', index: 5, step: 1 },
  { key: 'a', index: 0, step: 0.05 },
  { key: 'd', index: 3, step: 0.05 },
  { key: 'c', index: 2, step: 0.05 },
  { key: 'b', index: 1, step: 0.05 },
] as const

/** One editable field of the grid. */
export type TransformField = (typeof TRANSFORM_FIELDS)[number]

/** Drafts, keyed by component index; a missing entry is unchanged. */
export type TransformDrafts = Readonly<Record<number, Affine>>

/**
 * A signature of the components the grid is editing.
 *
 * A reload that changes the component list, the layer or the glyph must drop
 * every draft rather than apply numbers aimed at a different component, so the
 * grid resets whenever this string changes.
 */
export function componentsSignature(
  target: LayerTarget,
  components: readonly ComponentView[],
): string {
  return JSON.stringify([
    target.doc,
    target.glyph,
    target.layer,
    components.map(({ component }) => [component.base, component.transform]),
  ])
}

/** The transform a row currently shows: its draft, else the stored one. */
export function draftTransform(
  drafts: TransformDrafts,
  components: readonly ComponentView[],
  index: number,
): Affine {
  return (
    drafts[index] ??
    ((components[index]?.component.transform ?? [1, 0, 0, 1, 0, 0]) as Affine)
  )
}

/** The drafts after one field edit; an edit back to the stored value clears it. */
export function editDraft(
  drafts: TransformDrafts,
  components: readonly ComponentView[],
  index: number,
  field: TransformField,
  value: number,
): TransformDrafts {
  const stored = components[index]?.component.transform as Affine | undefined
  if (!stored) return drafts
  const next = [...draftTransform(drafts, components, index)] as Affine
  next[field.index] = value
  const result = { ...drafts }
  if (next.every((coefficient, at) => coefficient === stored[at]))
    delete result[index]
  else result[index] = next
  return result
}

/** Whether any component carries a staged change. */
export function hasDrafts(drafts: TransformDrafts): boolean {
  return Object.keys(drafts).length > 0
}

/**
 * The SVG transform that moves a component's *already transformed* path from
 * where the document has it to where the draft puts it.
 *
 * `ComponentView.path` is the base outline with the stored transform applied, so
 * previewing a different transform means undoing the stored one first. A stored
 * transform that cannot be undone — a collapsed scale — leaves the path where it
 * is instead of drawing `NaN`.
 */
export function previewMatrix(stored: Affine, draft: Affine): Affine {
  if (!isInvertible(stored)) return [1, 0, 0, 1, 0, 0]
  return composeAffine(draft, invertAffine(stored))
}

/**
 * Why a staged set cannot be applied, or `null` when it can.
 *
 * The two failures name their own `cjkConstruction.errors.*` string, so the grid
 * reports the reason instead of a disabled button with no explanation.
 */
export function draftProblem(
  drafts: TransformDrafts,
): 'empty' | 'transformInvalid' | 'transformCollapsed' | null {
  const values = Object.values(drafts)
  if (values.length === 0) return 'empty'
  if (values.some((draft) => draft.some((value) => !Number.isFinite(value))))
    return 'transformInvalid'
  // A singular transform erases the component's ink without deleting it, which
  // is never what positioning a radical means.
  if (values.some((draft) => !isInvertible(draft))) return 'transformCollapsed'
  return null
}

/** Every staged component transform as one undoable command. */
export function applyDraftsCommand(
  target: LayerTarget,
  components: readonly ComponentView[],
  drafts: TransformDrafts,
): Command {
  const commands = Object.entries(drafts)
    .map(([key, transform]) => [Number(key), transform] as const)
    .sort(([left], [right]) => left - right)
    .flatMap(([index, transform]) => {
      const stored = components[index]?.component
      if (!stored) return []
      const component: Component = { ...stored, transform }
      return [setComponentCommand(target, index, component)]
    })
  return { Batch: commands }
}
