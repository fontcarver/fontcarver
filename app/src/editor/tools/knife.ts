/**
 * The Knife tool (design spec §5).
 *
 * The stroke is a straight line; the crossings, the split and the closing
 * chords are all computed locally in `cutPath.ts`, and the result reaches the
 * kernel as one `Batch` of `DeleteContours` plus the pieces — one undo entry
 * called "Cut". Doing the geometry here rather than through
 * `InsertPointOnSegment` + `SplitContour` is what makes a closed shape come
 * back as closed pieces instead of one long open path.
 */

import type { Command, Contour } from '../../api'
import type { Point2 } from '../../canvas/Viewport'

import { cutContours, cutPoints } from '../cutPath'
import { toolCursor } from '../cursors'
import type { LayerTarget } from '../pointCommands'

import type { ToolId } from './registry'
import {
  constrainVector,
  NO_OUTCOME,
  NO_TOOL_OVERLAY,
  type EditorTool,
  type ToolContext,
  type ToolOutcome,
  type ToolOverlay,
  type ToolPointer,
} from './toolTypes'

export class KnifeTool implements EditorTool {
  readonly id: ToolId = 'knife'

  private from: Point2 | null = null
  private to: Point2 | null = null

  get busy(): boolean {
    return this.from !== null
  }

  pointerDown(pointer: ToolPointer, context: ToolContext): ToolOutcome {
    if (pointer.button !== 0) return NO_OUTCOME
    this.from = context.snap(pointer.font, pointer.modifiers.mod).point
    this.to = this.from
    return { handled: true }
  }

  pointerMove(pointer: ToolPointer, context: ToolContext): ToolOutcome {
    if (!this.from) return NO_OUTCOME
    this.to = this.endPoint(pointer, context)
    return { handled: true }
  }

  pointerUp(pointer: ToolPointer, context: ToolContext): ToolOutcome {
    const from = this.from
    if (!from) return NO_OUTCOME
    const to = this.endPoint(pointer, context)
    this.from = null
    this.to = null
    const target = context.target
    if (!target) return { handled: true }
    const result = cutContours(context.contours, from, to)
    if (result.removed.length === 0) return { handled: true }
    return {
      apply: {
        command: cutCommand(target, result.removed, result.added),
        label: context.labels.cut,
      },
      handled: true,
    }
  }

  finish(): ToolOutcome {
    this.from = null
    this.to = null
    return NO_OUTCOME
  }

  overlay(context: ToolContext): ToolOverlay {
    const { from, to } = this
    if (!from || !to) return NO_TOOL_OVERLAY
    return {
      ...NO_TOOL_OVERLAY,
      lines: [{ from, to, dashed: true }],
      marks: cutPoints(context.contours, from, to),
    }
  }

  cursor(): string {
    return toolCursor('knife')
  }

  private endPoint(pointer: ToolPointer, context: ToolContext): Point2 {
    const from = this.from
    const at = context.snap(pointer.font, pointer.modifiers.mod).point
    return from && pointer.modifiers.shift ? constrainVector(from, at) : at
  }
}

/** Replacing the cut contours with their pieces, as one undo entry. */
export function cutCommand(
  target: LayerTarget,
  removed: readonly number[],
  added: readonly Contour[],
): Command {
  return {
    Batch: [
      {
        DeleteContours: {
          glyph: target.glyph,
          layer: target.layer,
          contours: [...removed],
        },
      },
      ...added.map<Command>((contour) => ({
        AddContour: { glyph: target.glyph, layer: target.layer, contour },
      })),
    ],
  }
}
