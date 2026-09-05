/**
 * What the editor's keyboard and menu actions *decide*.
 *
 * Each of these is a pure function from the current outline and selection to a
 * command or a new selection; the controller keeps the asynchronous half —
 * applying, reporting and reconciling. Splitting them this way means the
 * grammar ("Tab walks the on-curve points", "Alt converts only a line") can be
 * read and tested without a gesture, a transport or a canvas.
 */

import type { Command, PointRef } from '../api'
import type { Point2, Viewport } from '../canvas/Viewport'
import type { SceneMetrics } from '../views/glyph/glyphScene'

import type { EditorLabels, EditorSource } from './editorTypes'
import type { Modifiers } from './gestures/drag'
import type { DragGesture } from './gestures/drag'
import type { Hit } from './hitTest'
import {
  nearestSplitParameter,
  type EditOutline,
  type EditSegment,
} from './outlineGeometry'
import {
  convertSegmentCommand,
  insertPointCommand,
  type LayerTarget,
} from './pointCommands'
import {
  onlyTarget,
  selectedPointRefs,
  selectionOf,
  type EditTarget,
  type Selection,
} from './selection'
import type { SnapInput } from './snapping'

/** Metrics used before the first layer has loaded. */
const FALLBACK_METRICS: SceneMetrics = {
  upem: 1000,
  ascender: 800,
  descender: -200,
  xHeight: 500,
  capHeight: 700,
  italicAngle: 0,
  width: 0,
  lsb: null,
  rsb: null,
  verticalOrigin: null,
  verticalWidth: null,
}

/** Every point of the layer, nodes and handles alike. */
export function allTargets(outline: EditOutline): Selection {
  const targets: EditTarget[] = outline.points.map((point) =>
    point.onCurve
      ? { kind: 'node', contour: point.ref.contour, point: point.ref.point }
      : {
          kind: 'handle',
          contour: point.ref.contour,
          point: point.ref.point,
        },
  )
  return selectionOf(targets)
}

/**
 * Tab and Shift+Tab walk the on-curve points in stored order, wrapping at
 * either end; anything other than a single node starts from the beginning.
 */
export function cycledSelection(
  outline: EditOutline,
  selection: Selection,
  direction: 1 | -1,
): Selection | null {
  const nodes = outline.points.filter((point) => point.onCurve)
  if (nodes.length === 0) return null
  const current = onlyTarget(selection)
  const index =
    current && current.kind === 'node'
      ? nodes.findIndex(
          (node) =>
            node.ref.contour === current.contour &&
            node.ref.point === current.point,
        )
      : -1
  const next = nodes[(index + direction + nodes.length * 2) % nodes.length]
  if (!next) return null
  return selectionOf([
    { kind: 'node', contour: next.ref.contour, point: next.ref.point },
  ])
}

/** One geometric segment of a layer, by contour and index. */
export function segmentAt(
  outline: EditOutline,
  contour: number,
  index: number,
): EditSegment | undefined {
  return outline.segments.find(
    (candidate) => candidate.contour === contour && candidate.index === index,
  )
}

/**
 * What a modified click on a segment does, if anything.
 *
 * Alt converts a *line* to a curve; on a curve it means "reshape keeping the
 * handle angles" (§4.3), and converting a curve to a curve would record an
 * empty undo entry and dirty the document for nothing. Shift inserts a node at
 * the nearest extremum or inflection rather than under the pointer.
 */
export function segmentShortcut(
  target: LayerTarget,
  labels: EditorLabels,
  outline: EditOutline,
  hit: Hit,
  modifiers: Modifiers,
): { command: Command; label: string } | null {
  if (hit.target.kind !== 'segment') return null
  const { contour, segment: index } = hit.target
  const segment = segmentAt(outline, contour, index)
  if (modifiers.alt && segment?.kind === 'line')
    return {
      command: convertSegmentCommand(target, contour, index, 'Curve'),
      label: labels.convertSegment,
    }
  if (modifiers.shift) {
    const clicked = Math.min(0.999, Math.max(0.001, hit.t ?? 0.5))
    const t = segment ? nearestSplitParameter(segment, clicked) : clicked
    return {
      command: insertPointCommand(
        target,
        contour,
        index,
        Math.min(0.999, Math.max(0.001, t)),
      ),
      label: labels.insertPoint,
    }
  }
  return null
}

/** Everything snapping looks at, assembled for the current gesture. */
export function buildSnapInput(input: {
  outline: EditOutline
  source: EditorSource | null
  selection: Selection
  drag: DragGesture | null
  labels: EditorLabels
  viewport: Viewport
}): SnapInput {
  // What is being dragged cannot snap to itself.
  const exclude = new Set(
    selectedPointRefs(input.selection).map(
      (ref) => `${ref.contour}:${ref.point}`,
    ),
  )
  for (const ref of input.drag?.plan.refs ?? [])
    exclude.add(`${ref.contour}:${ref.point}`)
  return {
    outline: input.outline,
    anchors: input.source?.outline.anchors ?? [],
    guides: input.source?.guides ?? [],
    metrics: input.source?.metrics ?? FALLBACK_METRICS,
    names: input.labels.snap,
    viewport: input.viewport,
    exclude,
  }
}

/** The stored points a nudge or a delete acts on. */
export function actionRefs(selection: Selection): PointRef[] {
  return selectedPointRefs(selection)
}

/** A canvas-space point, spelled out for readability at call sites. */
export type CanvasPoint = Point2
