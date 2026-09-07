/**
 * The Pen tool (design spec §5).
 *
 * Points accumulate locally with a live preview — a rubber band to the pointer,
 * handles that follow a press-drag — and reach the kernel exactly once: one
 * `AddContour` when the path closes or finishes, so a whole outline is one
 * undo entry. Picking a path up at one of its ends extends it instead, which
 * is a `DeleteContours` + `AddContour` batch under the label "Extend path".
 */

import type { Command, Contour } from '../../api'
import type { Point2 } from '../../canvas/Viewport'

import { toolCursor } from '../cursors'
import type { Modifiers } from '../gestures/drag'
import { HIT_RADII } from '../hitTest'
import { handlePolar, hudText } from '../hud'
import type { LayerCurves } from '../pathGeometry'
import type { LayerTarget } from '../pointCommands'

import { nodesOfContour, penContour, penNode, type PenNode } from './penPath'
import type { ToolId } from './registry'
import {
  constrainVector,
  movedEnough,
  NO_OUTCOME,
  NO_TOOL_OVERLAY,
  type EditorTool,
  type ToolContext,
  type ToolKey,
  type ToolOutcome,
  type ToolOverlay,
  type ToolPointer,
} from './toolTypes'

/** The press that is shaping the newest node, while the button is down. */
interface PenPress {
  /** Where the press started, in font units. */
  origin: Point2
  /** Whether the press has become a handle drag. */
  dragging: boolean
  /** Whether releasing this press closes the path. */
  closing: boolean
}

/** Which stored contour the pen picked up, when it is extending one. */
interface PenExtend {
  contour: number
  /** The source contour's name and id, which the extended path keeps. */
  identity: { name: string | null; id: string | null }
}

export class PenTool implements EditorTool {
  readonly id: ToolId = 'pen'

  private nodes: PenNode[] = []
  private press: PenPress | null = null
  private extend: PenExtend | null = null
  private pointer: Point2 | null = null
  private spaceHeld = false

  get busy(): boolean {
    return this.press !== null
  }

  /** The nodes drawn so far; read by tests and by the overlay. */
  get nodeCount(): number {
    return this.nodes.length
  }

  pointerDown(pointer: ToolPointer, context: ToolContext): ToolOutcome {
    if (pointer.button !== 0) return NO_OUTCOME
    const hit = context.hitTest(pointer.screen)
    // ⇧+click on a segment inserts a node there, exactly as the Select tool
    // does: the pen shares the §4.3 segment grammar.
    if (hit && this.nodes.length === 0) {
      const shortcut = context.segmentShortcut(hit, pointer.modifiers)
      if (shortcut) return { apply: shortcut, handled: true }
    }

    const at = context.snap(pointer.font, pointer.modifiers.mod).point

    if (this.nodes.length === 0) {
      const pickup = openEnd(context, hit)
      if (pickup) {
        this.nodes = nodesOfContour(pickup.contour, pickup.end)
        this.extend = {
          contour: pickup.index,
          identity: { name: pickup.contour.name, id: pickup.contour.id },
        }
        this.press = { origin: at, dragging: false, closing: false }
        return { handled: true }
      }
      this.nodes = [penNode(at)]
      this.extend = null
      this.press = { origin: at, dragging: false, closing: false }
      return { handled: true }
    }

    if (this.overFirstNode(pointer.screen, context)) {
      // The closing click may still drag, which shapes the first node's
      // incoming handle, so the commit waits for the pointer to come up.
      this.press = { origin: at, dragging: false, closing: true }
      return { handled: true }
    }

    this.nodes.push(penNode(at))
    this.press = { origin: at, dragging: false, closing: false }
    return { handled: true }
  }

  pointerMove(pointer: ToolPointer, context: ToolContext): ToolOutcome {
    this.pointer = pointer.font
    const press = this.press
    if (!press) return { handled: this.nodes.length > 0 }
    const node = this.activeNode(press)
    if (!node) return { handled: true }

    if (this.spaceHeld) {
      // Space repositions the node the press is placing, handles and all.
      const at = context.snap(pointer.font, pointer.modifiers.mod).point
      const dx = at.x - node.point.x
      const dy = at.y - node.point.y
      node.point = at
      if (node.incoming)
        node.incoming = { x: node.incoming.x + dx, y: node.incoming.y + dy }
      if (node.outgoing)
        node.outgoing = { x: node.outgoing.x + dx, y: node.outgoing.y + dy }
      press.origin = at
      return { handled: true }
    }

    if (
      !press.dragging &&
      !movedEnough(press.origin, pointer.font, context.viewport.scale)
    )
      return { handled: true }
    press.dragging = true

    const target = pointer.modifiers.shift
      ? constrainVector(node.point, pointer.font)
      : pointer.font
    if (press.closing) {
      // Closing: the drag shapes the handle that arrives at the first node.
      node.incoming = target
      node.smooth = false
      return { handled: true }
    }
    node.outgoing = target
    // ⌥ sets the outgoing handle alone, leaving the incoming one where it is;
    // without it the pair is mirrored and the node is smooth.
    if (!pointer.modifiers.alt) {
      node.incoming = {
        x: 2 * node.point.x - target.x,
        y: 2 * node.point.y - target.y,
      }
      node.smooth = true
    }
    return { handled: true }
  }

  pointerUp(_pointer: ToolPointer, context: ToolContext): ToolOutcome {
    const press = this.press
    this.press = null
    if (!press) return NO_OUTCOME
    if (!press.closing) return { handled: true }
    return { ...this.commit(context, true), handled: true }
  }

  keyDown(key: ToolKey, context: ToolContext): ToolOutcome {
    if (key.key === 'Enter')
      return { ...this.commit(context, false), handled: true }
    if (key.key === 'Escape') {
      if (this.nodes.length >= 2)
        return { ...this.commit(context, false), handled: true }
      const had = this.nodes.length > 0
      this.reset()
      return { handled: had }
    }
    return NO_OUTCOME
  }

  claimsKey(key: string): boolean {
    // A path in progress owns both keys: Enter finishes it and Escape either
    // finishes or abandons it, and neither may reach the command registry.
    return (key === 'Enter' || key === 'Escape') && this.nodes.length > 0
  }

  setSpaceHeld(held: boolean): ToolOutcome {
    this.spaceHeld = held
    return { handled: this.press !== null }
  }

  pointerLeave(): ToolOutcome {
    // The rubber band follows the pointer, so it stops where the pointer left.
    return NO_OUTCOME
  }

  finish(context: ToolContext): ToolOutcome {
    this.press = null
    return this.commit(context, false)
  }

  overlay(context: ToolContext): ToolOverlay {
    if (this.nodes.length === 0) return NO_TOOL_OVERLAY
    const nodes = this.previewNodes()
    const contour = penContour(nodes, false, context.curves)
    const lines: { from: Point2; to: Point2 }[] = []
    const marks: Point2[] = []
    for (const node of nodes) {
      marks.push(node.point)
      if (node.incoming) lines.push({ from: node.point, to: node.incoming })
      if (node.outgoing) lines.push({ from: node.point, to: node.outgoing })
    }
    for (const node of nodes)
      for (const handle of [node.incoming, node.outgoing])
        if (handle) marks.push(handle)
    return {
      contours: contour ? [contour] : [],
      lines,
      marks,
      labels: [],
      origin: null,
      hud: this.hud(context),
    }
  }

  cursor(
    pointer: { screen: Point2; modifiers: Modifiers } | null,
    context: ToolContext,
  ): string {
    if (!pointer) return toolCursor('pen')
    if (this.nodes.length > 0 && this.overFirstNode(pointer.screen, context))
      return toolCursor('penClose')
    const hit = context.hitTest(pointer.screen)
    if (this.nodes.length === 0 && openEnd(context, hit))
      return toolCursor('penStart')
    if (hit?.target.kind === 'segment') return toolCursor('penAdd')
    return toolCursor('pen')
  }

  /** The nodes plus the rubber-band node under the pointer. */
  private previewNodes(): PenNode[] {
    if (this.press || !this.pointer || this.nodes.length === 0)
      return this.nodes
    return [...this.nodes, penNode(this.pointer)]
  }

  private activeNode(press: PenPress): PenNode | undefined {
    return press.closing ? this.nodes[0] : this.nodes[this.nodes.length - 1]
  }

  private hud(context: ToolContext): string | null {
    const press = this.press
    if (!press?.dragging) return null
    const node = this.activeNode(press)
    const handle = press.closing ? node?.incoming : node?.outgoing
    if (!node || !handle) return null
    const polar = handlePolar(node.point, handle)
    return hudText(
      {
        kind: 'handle',
        delta: { x: handle.x - node.point.x, y: handle.y - node.point.y },
        angle: polar.angle,
        length: polar.length,
      },
      context.labels.hud,
    )
  }

  private overFirstNode(screen: Point2, context: ToolContext): boolean {
    const first = this.nodes[0]
    if (!first || this.nodes.length < 2) return false
    const at = context.viewport.toScreen(first.point)
    return Math.hypot(at.x - screen.x, at.y - screen.y) <= HIT_RADII.node
  }

  /** Commits the accumulated path, or drops it when there is too little. */
  private commit(context: ToolContext, closed: boolean): ToolOutcome {
    const target = context.target
    const nodes = this.nodes
    const extend = this.extend
    this.reset()
    if (!target || nodes.length < 2) return NO_OUTCOME
    const contour = penContour(nodes, closed, context.curves, extend?.identity)
    if (!contour) return NO_OUTCOME
    return {
      apply: extend
        ? {
            command: extendCommand(target, extend.contour, contour),
            label: context.labels.extendPath,
          }
        : {
            command: addContour(target, contour),
            label: context.labels.drawPath,
          },
    }
  }

  private reset(): void {
    this.nodes = []
    this.press = null
    this.extend = null
    this.pointer = null
  }
}

/** `AddContour` for one finished path. */
export function addContour(target: LayerTarget, contour: Contour): Command {
  return {
    AddContour: { glyph: target.glyph, layer: target.layer, contour },
  }
}

/** Replacing an open contour with its extension, as one undo entry. */
function extendCommand(
  target: LayerTarget,
  index: number,
  contour: Contour,
): Command {
  return {
    Batch: [
      {
        DeleteContours: {
          glyph: target.glyph,
          layer: target.layer,
          contours: [index],
        },
      },
      addContour(target, contour),
    ],
  }
}

/** The open contour end a hit picked up, when the hit is one. */
function openEnd(
  context: ToolContext,
  hit: ReturnType<ToolContext['hitTest']>,
): { index: number; contour: Contour; end: 'start' | 'end' } | null {
  if (hit?.target.kind !== 'node') return null
  const index = hit.target.contour
  const contour = context.contours[index]
  if (!contour || contour.closed) return null
  const onCurve = contour.points
    .map((point, position) => ({ point, position }))
    .filter((entry) => entry.point.kind !== 'OffCurve')
  const first = onCurve[0]
  const last = onCurve[onCurve.length - 1]
  if (first && hit.target.point === first.position)
    return { index, contour, end: 'start' }
  if (last && hit.target.point === last.position)
    return { index, contour, end: 'end' }
  return null
}

/** The curve flavour a pen path uses; exported for the tests. */
export type { LayerCurves }
