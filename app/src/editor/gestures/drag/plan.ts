/**
 * What a pointer-down drags.
 *
 * The plan is decided once, on pointer-down, and never changes for the rest of
 * the gesture: which stored points, anchors and components travel, whether the
 * handles of a node come with it, and what the resulting undo entry is called.
 * Keeping that decision in a pure function means "what will this drag move?"
 * is answerable from a hit and a modifier set alone.
 */

import type {
  Anchor,
  ComponentView,
  Contour,
  DocId,
  HandleMode,
  PointRef,
} from '../../../api'
import type { Point2 } from '../../../canvas/Viewport'
import type { Scheduler } from '../../editSession'
import type { Hit } from '../../hitTest'
import { buildEditOutline } from '../../outlineGeometry'
import {
  isSelected,
  selectedIndexes,
  selectedNodeRefs,
  selectedPointRefs,
  type Selection,
} from '../../selection'

/** The modifier keys, named by what they mean rather than by platform. */
export interface Modifiers {
  /** Constrain to 0/45/90°, or extend a selection. */
  shift: boolean
  /** "The other behaviour": ignore handles, keep an angle, from the centre. */
  alt: boolean
  /** Command on macOS, Control elsewhere; suspends snapping. */
  mod: boolean
}

/** No modifier held. */
export const NO_MODIFIERS: Modifiers = { shift: false, alt: false, mod: false }

/** What a drag moves and what its undo entry is called. */
export interface DragPlan {
  kind: 'points' | 'handle' | 'segment' | 'anchor' | 'component'
  /** Stored points the command moves. */
  refs: PointRef[]
  /** Anchor indexes carried by the gesture. */
  anchors: number[]
  /** Component indexes carried by the gesture. */
  components: number[]
  handleMode: HandleMode
  /** Undo label, already translated. */
  label: string
}

/** How a drag names its undo entry; supplied by the caller so the editor
 * modules stay free of i18n wiring. */
export interface DragLabels {
  movePoints: (count: number) => string
  /** Used when the moved set is handles only. */
  moveHandles: (count: number) => string
  moveHandle: string
  reshapeSegment: string
  moveAnchor: string
  moveComponent: string
}

/** A snap decision for the point under the pointer. */
export interface SnapResult {
  /** The delta to use instead of the requested one. */
  delta: Point2
  /** Name of what was snapped to, for the HUD. */
  label: string | null
}

/** Everything a drag needs about the glyph it edits. */
export interface DragContext {
  doc: DocId
  glyph: string
  layer: string
  contours: readonly Contour[]
  anchors: readonly Anchor[]
  components: readonly ComponentView[]
  selection: Selection
  labels: DragLabels
  /** Applies snapping to a proposed delta; omitted or `null` disables it. */
  snap?: (delta: Point2, moved: readonly Point2[]) => SnapResult
  scheduler?: Scheduler
  /** Where a failed kernel call is reported. */
  onError?: (error: unknown) => void
  /** A gesture still closing, which this one's `edit_begin` waits for. */
  after?: Promise<unknown>
}

/** The map key a stored point index is deduplicated by. */
export function refKey(ref: PointRef): string {
  return `${ref.contour}:${ref.point}`
}

/** The listed points, deduplicated and in stored order. */
export function uniqueRefs(refs: readonly PointRef[]): PointRef[] {
  const seen = new Map<string, PointRef>()
  for (const ref of refs) seen.set(refKey(ref), ref)
  return [...seen.values()].sort(
    (a, b) => a.contour - b.contour || a.point - b.point,
  )
}

/**
 * Decides what a pointer-down on `hit` drags.
 *
 * Dragging something that is already selected moves the whole selection;
 * dragging something outside it moves that one target, which is the behaviour
 * every outline editor shares and the reason a click-drag never needs a click
 * first.
 */
export function planDrag(
  context: DragContext,
  hit: Hit,
  modifiers: Modifiers,
): DragPlan | null {
  const { selection, labels } = context
  const target = hit.target
  const inSelection = isSelected(selection, target)

  if (target.kind === 'node') {
    const refs = inSelection
      ? selectedPointRefs(selection)
      : [{ contour: target.contour, point: target.point }]
    const nodes = inSelection ? selectedNodeRefs(selection).length : 1
    return {
      kind: 'points',
      refs,
      anchors: inSelection ? selectedIndexes(selection, 'anchor') : [],
      components: inSelection ? selectedIndexes(selection, 'component') : [],
      // Alt moves the node alone; the default carries its handles with it.
      // Ctrl+Alt is a different gesture — the proportional nudge — and there
      // the node keeps its handles while the neighbours' handles scale.
      handleMode:
        modifiers.alt && !modifiers.mod ? 'PointsOnly' : 'WithHandles',
      // A selection of handles alone is named for what it is: counting nodes
      // would label it "Move 0 points".
      label:
        nodes > 0 ? labels.movePoints(nodes) : labels.moveHandles(refs.length),
    }
  }

  if (target.kind === 'handle')
    return {
      kind: 'handle',
      refs: [{ contour: target.contour, point: target.point }],
      anchors: [],
      components: [],
      handleMode: 'PointsOnly',
      label: labels.moveHandle,
    }

  if (target.kind === 'segment') {
    const segment = context.contours[target.contour]
    if (!segment) return null
    return {
      kind: 'segment',
      refs: segmentRefs(context, target.contour, target.segment),
      anchors: [],
      components: [],
      handleMode: 'WithHandles',
      label: labels.reshapeSegment,
    }
  }

  if (target.kind === 'anchor')
    return {
      kind: 'anchor',
      refs: [],
      anchors: inSelection
        ? selectedIndexes(selection, 'anchor')
        : [target.index],
      components: [],
      handleMode: 'PointsOnly',
      label: labels.moveAnchor,
    }

  if (target.kind === 'component')
    return {
      kind: 'component',
      refs: [],
      anchors: [],
      components: inSelection
        ? selectedIndexes(selection, 'component')
        : [target.index],
      handleMode: 'PointsOnly',
      label: labels.moveComponent,
    }

  return null
}

/**
 * The stored points a segment drag moves: the controls of a curve, or both
 * endpoints of a straight line, which is what "drag the segment" means when
 * there is no curvature to reshape.
 */
function segmentRefs(
  context: DragContext,
  contourIndex: number,
  segmentIndex: number,
): PointRef[] {
  const contour = context.contours[contourIndex]
  if (!contour) return []
  const outline = buildEditOutline(context.contours)
  const segment = outline.segments.find(
    (candidate) =>
      candidate.contour === contourIndex && candidate.index === segmentIndex,
  )
  if (!segment) return []
  if (segment.kind !== 'line') return uniqueRefs(segment.controlRefs)
  return uniqueRefs(
    [segment.fromRef, segment.toRef].filter(
      (ref): ref is PointRef => ref !== null,
    ),
  )
}
