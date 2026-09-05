/**
 * What the glyph editor can select, and the pure operations over a selection.
 *
 * A selection is a map from a stable key to the target it names, which makes
 * membership tests, `Shift`-extend and `Ctrl`-toggle trivial and keeps the
 * value immutable so it can live in the editor store without cloning traps.
 * Nodes and handles are addressed by their *stored* indexes (`PointRef`), the
 * same coordinates every editing command takes, so nothing has to translate
 * between a drawn point and the command that moves it.
 */

import type { PointRef } from '../api'

/** Whether a guide belongs to this layer or to the whole master. */
export type GuideScope = 'layer' | 'master'

/** One selectable thing on the canvas. */
export type EditTarget =
  | { readonly kind: 'node'; readonly contour: number; readonly point: number }
  | {
      readonly kind: 'handle'
      readonly contour: number
      readonly point: number
    }
  | {
      readonly kind: 'segment'
      readonly contour: number
      readonly segment: number
    }
  | { readonly kind: 'component'; readonly index: number }
  | { readonly kind: 'anchor'; readonly index: number }
  | {
      readonly kind: 'guide'
      readonly index: number
      readonly scope: GuideScope
    }

/** An immutable selection, keyed by `targetKey`. */
export type Selection = ReadonlyMap<string, EditTarget>

/** The selection of a glyph nothing has been clicked in yet. */
export const EMPTY_SELECTION: Selection = new Map()

/** Stable identity of a target, used as the selection key. */
export function targetKey(target: EditTarget): string {
  switch (target.kind) {
    case 'node':
    case 'handle':
      return `${target.kind}:${target.contour}:${target.point}`
    case 'segment':
      return `segment:${target.contour}:${target.segment}`
    case 'guide':
      return `guide:${target.scope}:${target.index}`
    default:
      return `${target.kind}:${target.index}`
  }
}

/** Whether `target` is part of `selection`. */
export function isSelected(selection: Selection, target: EditTarget): boolean {
  return selection.has(targetKey(target))
}

/** A selection holding exactly the listed targets. */
export function selectionOf(targets: readonly EditTarget[]): Selection {
  return new Map(targets.map((target) => [targetKey(target), target]))
}

/** A selection holding one target. */
export function selectOne(target: EditTarget): Selection {
  return selectionOf([target])
}

/** `Shift`-click: adds a target, keeping everything already selected. */
export function extendSelection(
  selection: Selection,
  target: EditTarget,
): Selection {
  const next = new Map(selection)
  next.set(targetKey(target), target)
  return next
}

/** `Ctrl`-click: adds a target, or removes it when it is already selected. */
export function toggleSelection(
  selection: Selection,
  target: EditTarget,
): Selection {
  const next = new Map(selection)
  const key = targetKey(target)
  if (!next.delete(key)) next.set(key, target)
  return next
}

/** The union of two selections, `b` winning on identical keys. */
export function unionSelection(a: Selection, b: Selection): Selection {
  return new Map([...a, ...b])
}

/** Whether two selections name the same targets. */
export function sameSelection(a: Selection, b: Selection): boolean {
  if (a === b) return true
  if (a.size !== b.size) return false
  for (const key of a.keys()) if (!b.has(key)) return false
  return true
}

/** Every selected node and handle as a stored point index, in stored order. */
export function selectedPointRefs(selection: Selection): PointRef[] {
  const refs: PointRef[] = []
  for (const target of selection.values())
    if (target.kind === 'node' || target.kind === 'handle')
      refs.push({ contour: target.contour, point: target.point })
  refs.sort((a, b) => a.contour - b.contour || a.point - b.point)
  return refs
}

/** Every selected on-curve node as a stored point index, in stored order. */
export function selectedNodeRefs(selection: Selection): PointRef[] {
  const refs: PointRef[] = []
  for (const target of selection.values())
    if (target.kind === 'node')
      refs.push({ contour: target.contour, point: target.point })
  refs.sort((a, b) => a.contour - b.contour || a.point - b.point)
  return refs
}

/** Indexes of the selected targets of one indexed kind, ascending. */
export function selectedIndexes(
  selection: Selection,
  kind: 'component' | 'anchor',
): number[] {
  const indexes: number[] = []
  for (const target of selection.values())
    if (target.kind === kind) indexes.push(target.index)
  return indexes.sort((a, b) => a - b)
}

/** How many targets of each kind a selection holds. */
export interface SelectionCounts {
  nodes: number
  handles: number
  segments: number
  components: number
  anchors: number
  guides: number
  total: number
}

/** Counts the selection by kind; the Info box switches on these. */
export function selectionCounts(selection: Selection): SelectionCounts {
  const counts: SelectionCounts = {
    nodes: 0,
    handles: 0,
    segments: 0,
    components: 0,
    anchors: 0,
    guides: 0,
    total: selection.size,
  }
  for (const target of selection.values()) {
    if (target.kind === 'node') counts.nodes += 1
    else if (target.kind === 'handle') counts.handles += 1
    else if (target.kind === 'segment') counts.segments += 1
    else if (target.kind === 'component') counts.components += 1
    else if (target.kind === 'anchor') counts.anchors += 1
    else counts.guides += 1
  }
  return counts
}

/** The single selected target, or `null` when the selection is not exactly one. */
export function onlyTarget(selection: Selection): EditTarget | null {
  if (selection.size !== 1) return null
  return [...selection.values()][0] ?? null
}
