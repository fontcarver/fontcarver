/**
 * Single-shot editing commands.
 *
 * Everything here is one `doc.apply`, which is one undo entry: a smooth
 * toggle, a delete, an inserted point, a converted segment and every Info box
 * field commit. Gestures that stream (drag, nudge) live in `gestures/`.
 */

import {
  type Anchor,
  type Command,
  type Component,
  type Contour,
  type DeleteMode,
  type DocId,
  type Guide,
  type HandleMode,
  type PointRef,
  type SegmentKind,
} from '../api'

import { applyCommand } from './editSession'
import { pointAt } from './outlineGeometry'
import type { GuideScope } from './selection'

/** The glyph and layer every outline command names. */
export interface LayerTarget {
  doc: DocId
  glyph: string
  layer: string
}

/** `SetSmooth` for the listed nodes. */
export function smoothCommand(
  target: LayerTarget,
  points: readonly PointRef[],
  smooth: boolean,
  levelHandles: boolean,
): Command {
  return {
    SetSmooth: {
      glyph: target.glyph,
      layer: target.layer,
      points: [...points],
      smooth,
      level_handles: levelHandles,
    },
  }
}

/**
 * The flag a toggle should write: a selection containing any corner becomes
 * smooth, a selection that is already all smooth becomes corners.
 */
export function nextSmoothFlag(
  contours: readonly Contour[],
  points: readonly PointRef[],
): boolean {
  return points.some((ref) => pointAt(contours, ref)?.smooth !== true)
}

/** `DeletePoints` with the policy the modifiers asked for. */
export function deleteCommand(
  target: LayerTarget,
  points: readonly PointRef[],
  mode: DeleteMode,
): Command {
  return {
    DeletePoints: {
      glyph: target.glyph,
      layer: target.layer,
      points: [...points],
      mode,
    },
  }
}

/** `InsertPointOnSegment` at a parametric position. */
export function insertPointCommand(
  target: LayerTarget,
  contour: number,
  segment: number,
  t: number,
): Command {
  return {
    InsertPointOnSegment: {
      glyph: target.glyph,
      layer: target.layer,
      contour,
      segment,
      t,
    },
  }
}

/** `ConvertSegment` between a line and a curve. */
export function convertSegmentCommand(
  target: LayerTarget,
  contour: number,
  segment: number,
  to: SegmentKind,
): Command {
  return {
    ConvertSegment: {
      glyph: target.glyph,
      layer: target.layer,
      contour,
      segment,
      to,
    },
  }
}

/** `SetPointPosition`: the Info box committing a node's X or Y. */
export function setPositionCommand(
  target: LayerTarget,
  point: PointRef,
  x: number,
  y: number,
): Command {
  return {
    SetPointPosition: { glyph: target.glyph, layer: target.layer, point, x, y },
  }
}

/** `MovePoints`: the Info box committing a handle's ΔX, ΔY, length or angle. */
export function movePointsCommand(
  target: LayerTarget,
  points: readonly PointRef[],
  dx: number,
  dy: number,
  handles: HandleMode,
): Command {
  return {
    MovePoints: {
      glyph: target.glyph,
      layer: target.layer,
      points: [...points],
      delta: [dx, dy],
      handles,
    },
  }
}

/** `TransformSelection`: the Info box committing a multi-selection bounding box. */
export function transformSelectionCommand(
  target: LayerTarget,
  selection: {
    points: readonly PointRef[]
    components: readonly number[]
    anchors: readonly number[]
  },
  affine: [number, number, number, number, number, number],
): Command {
  return {
    TransformSelection: {
      glyph: target.glyph,
      layer: target.layer,
      points: [...selection.points],
      components: [...selection.components],
      anchors: [...selection.anchors],
      affine,
    },
  }
}

/** `SetAnchor`: the Info box committing an anchor's name or position. */
export function setAnchorCommand(
  target: LayerTarget,
  index: number,
  anchor: Anchor,
): Command {
  return {
    SetAnchor: { glyph: target.glyph, layer: target.layer, index, anchor },
  }
}

/** `SetComponent`: the Info box committing a component's offset or scale. */
export function setComponentCommand(
  target: LayerTarget,
  index: number,
  component: Component,
): Command {
  return {
    SetComponent: {
      glyph: target.glyph,
      layer: target.layer,
      index,
      component,
    },
  }
}

/** `SetGuide`: the Info box committing a guide's position, angle or name. */
export function setGuideCommand(
  target: LayerTarget,
  scope: GuideScope,
  master: string | null,
  index: number,
  guide: Guide,
): Command {
  return scope === 'layer'
    ? {
        SetGuide: {
          glyph: target.glyph,
          layer: target.layer,
          master: null,
          index,
          guide,
        },
      }
    : {
        SetGuide: {
          glyph: null,
          layer: null,
          master,
          index,
          guide,
        },
      }
}

/** `SetMetrics`: the Info box committing width, LSB or RSB. */
export function setMetricsCommand(
  target: LayerTarget,
  values: {
    width?: number | null
    lsb?: number | null
    rsb?: number | null
  },
): Command {
  return {
    SetMetrics: {
      glyph: target.glyph,
      layer: target.layer,
      width: values.width ?? null,
      lsb: values.lsb ?? null,
      rsb: values.rsb ?? null,
    },
  }
}

/** Applies one command as its own undo entry. */
export function commit(
  target: LayerTarget,
  command: Command,
  label: string,
): Promise<void> {
  return applyCommand(target.doc, command, label)
}
