/**
 * The Magnet tool: drag a layer guide onto nodes and they follow it.
 *
 * Pressing a layer guide picks it up. While it is dragged, every on-curve node
 * within the magnet radius of where the guide now lies is marked, and releasing
 * writes one undo entry that moves the guide and binds those nodes to it — a
 * Power Guide. Holding Alt projects the caught nodes onto the guide first, so
 * they sit exactly on it instead of keeping their offsets.
 *
 * Only an ordinary layer guide can own nodes, which is the kernel's rule
 * (`Layer.linked_guides`), so a master guide reports why it cannot be used.
 */

import type { Command, Guide, PointRef } from '../../api'
import type { Point2 } from '../../canvas/Viewport'
import { i18n } from '../../i18n'
import { setGuideCommand } from '../pointCommands'

import {
  NO_OUTCOME,
  NO_TOOL_OVERLAY,
  type EditorTool,
  type ToolContext,
  type ToolKey,
  type ToolOutcome,
  type ToolOverlay,
  type ToolPointer,
} from './toolTypes'

/** How close a node must come to the dragged guide to be caught, in CSS pixels. */
export const MAGNET_RADIUS = 12
/** Largest number of nodes one magnet gesture binds. */
export const MAGNET_LIMIT = 256

/** A guide being dragged, with where it started. */
interface MagnetDrag {
  index: number
  guide: Guide
  /** Font-space position of the press. */
  from: Point2
  /** Font-space position of the pointer now. */
  to: Point2
  moved: boolean
  project: boolean
}

/** The guide as the drag currently places it. */
function dragged(drag: MagnetDrag): Guide {
  return {
    ...drag.guide,
    x: drag.guide.x + (drag.to.x - drag.from.x),
    y: drag.guide.y + (drag.to.y - drag.from.y),
  }
}

/** Signed distance from a point to a guide along the guide's left normal. */
export function guideOffset(guide: Guide, point: Point2): number {
  const radians = (guide.angle * Math.PI) / 180
  const sin = Math.sin(radians)
  const cos = Math.cos(radians)
  return -(point.x - guide.x) * sin + (point.y - guide.y) * cos
}

/** On-curve nodes within `radius` font units of a guide, in stored order. */
export function caughtNodes(
  guide: Guide,
  points: readonly { ref: PointRef; x: number; y: number; onCurve: boolean }[],
  radius: number,
): PointRef[] {
  const caught: PointRef[] = []
  for (const point of points) {
    if (!point.onCurve) continue
    if (Math.abs(guideOffset(guide, point)) <= radius) caught.push(point.ref)
    if (caught.length >= MAGNET_LIMIT) break
  }
  return caught
}

export class MagnetTool implements EditorTool {
  readonly id = 'magnet' as const
  private drag: MagnetDrag | null = null
  /** Why the last press did nothing, shown in the cursor HUD. */
  private refused: string | null = null

  get busy(): boolean {
    return this.drag !== null
  }

  pointerDown(pointer: ToolPointer, context: ToolContext): ToolOutcome {
    this.drag = null
    this.refused = null
    if (pointer.button !== 0) return NO_OUTCOME
    const hit = context.hitTest(pointer.screen)
    const target = hit?.target.kind === 'guide' ? hit.target : null
    if (!target) {
      this.refused = i18n.t('construction.magnetNoGuide')
      return { handled: true }
    }
    if (target.scope !== 'layer') {
      this.refused = i18n.t('construction.magnetLayerOnly')
      return { handled: true }
    }
    const entry = (context.guides ?? []).find(
      (candidate) =>
        candidate.scope === 'layer' && candidate.index === target.index,
    )
    if (!entry || entry.guide.locked) {
      this.refused = i18n.t('construction.magnetLocked')
      return { handled: true }
    }
    this.drag = {
      index: target.index,
      guide: entry.guide,
      from: pointer.font,
      to: pointer.font,
      moved: false,
      project: pointer.modifiers.alt,
    }
    return { handled: true, editing: true }
  }

  pointerMove(pointer: ToolPointer, context: ToolContext): ToolOutcome {
    const drag = this.drag
    if (!drag) return NO_OUTCOME
    drag.to = pointer.font
    drag.project = pointer.modifiers.alt
    drag.moved =
      drag.moved ||
      Math.hypot(drag.to.x - drag.from.x, drag.to.y - drag.from.y) *
        context.viewport.scale >=
        1
    return { handled: true }
  }

  pointerUp(_pointer: ToolPointer, context: ToolContext): ToolOutcome {
    const drag = this.drag
    this.drag = null
    if (!drag) return NO_OUTCOME
    const target = context.target
    if (!target) return { handled: true, editing: false }
    const guide = dragged(drag)
    const radius = MAGNET_RADIUS / Math.max(context.viewport.scale, 1e-9)
    const points = caughtNodes(guide, context.outline.points, radius).filter(
      (ref) => context.canSelect?.({ kind: 'node', ...ref }) !== false,
    )
    const commands: Command[] = []
    if (drag.moved)
      commands.push(setGuideCommand(target, 'layer', null, drag.index, guide))
    if (points.length > 0) {
      if (drag.project)
        commands.push({
          NodeLinks: {
            ProjectToGuide: {
              glyph: target.glyph,
              layer: target.layer,
              guide_index: drag.index,
              points,
            },
          },
        })
      commands.push({
        NodeLinks: {
          LinkToGuide: {
            glyph: target.glyph,
            layer: target.layer,
            guide_index: drag.index,
            points,
          },
        },
      })
    }
    if (commands.length === 0) return { handled: true, editing: false }
    return {
      handled: true,
      editing: false,
      apply: {
        command: commands.length === 1 ? commands[0]! : { Batch: commands },
        label: i18n.t(
          points.length > 0
            ? 'construction.undo.magnet'
            : 'construction.undo.moveGuide',
        ),
      },
    }
  }

  keyDown(key: ToolKey): ToolOutcome {
    if (key.key !== 'Escape' || !this.drag) return NO_OUTCOME
    this.drag = null
    return { handled: true, editing: false }
  }

  claimsKey(key: string): boolean {
    return key === 'Escape' && this.drag !== null
  }

  finish(): ToolOutcome {
    this.drag = null
    this.refused = null
    return NO_OUTCOME
  }

  overlay(context: ToolContext): ToolOverlay {
    const drag = this.drag
    if (!drag)
      return this.refused
        ? { ...NO_TOOL_OVERLAY, hud: this.refused }
        : NO_TOOL_OVERLAY
    const guide = dragged(drag)
    const radians = (guide.angle * Math.PI) / 180
    const reach =
      (Math.max(context.viewport.width, context.viewport.height) + 100) /
      Math.max(context.viewport.scale, 1e-9)
    const along = { x: Math.cos(radians) * reach, y: Math.sin(radians) * reach }
    const radius = MAGNET_RADIUS / Math.max(context.viewport.scale, 1e-9)
    const points = caughtNodes(guide, context.outline.points, radius)
    const marks = points.map((ref) => {
      const point = context.outline.points.find(
        (candidate) =>
          candidate.ref.contour === ref.contour &&
          candidate.ref.point === ref.point,
      )
      return { x: point?.x ?? guide.x, y: point?.y ?? guide.y }
    })
    return {
      ...NO_TOOL_OVERLAY,
      lines: [
        {
          from: { x: guide.x - along.x, y: guide.y - along.y },
          to: { x: guide.x + along.x, y: guide.y + along.y },
          dashed: true,
        },
      ],
      marks,
      hud: i18n.t(
        drag.project ? 'construction.magnetSnapHud' : 'construction.magnetHud',
        { count: points.length },
      ),
    }
  }

  cursor(pointer: { screen: Point2 } | null, context: ToolContext): string {
    if (this.drag) return 'grabbing'
    if (!pointer) return 'crosshair'
    const hit = context.hitTest(pointer.screen)
    return hit?.target.kind === 'guide' && hit.target.scope === 'layer'
      ? 'grab'
      : 'crosshair'
  }
}
