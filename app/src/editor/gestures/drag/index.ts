/**
 * Dragging a selection with the Select tool.
 *
 * The gesture owns three things: what moves (planned once, on pointer-down,
 * in `plan.ts`), where it moves to (recomputed from the gesture origin on
 * every pointer move, after the modifier constraints in `deltas.ts` and
 * snapping have had their say) and the kernel protocol (one `edit_begin`,
 * throttled `edit_apply`s, one `edit_end`, which is one undo entry).
 *
 * The canvas draws from `preview`, a local copy of the outline that is updated
 * synchronously on every pointer move, so the outline never lags the pointer
 * even though the kernel is only told once per animation frame.
 */

import type { Command, PointRef } from '../../../api'
import type { Point2 } from '../../../canvas/Viewport'
import { EditSession } from '../../editSession'
import type { Hit } from '../../hitTest'
import {
  handlesOfNode,
  nodeOfHandle,
  oppositeHandle,
  pointAt,
} from '../../outlineGeometry'

import { perHandleCommand, stepCommand } from './commands'
import {
  constrainDelta,
  nudgeDeltas,
  perHandleDeltas,
  projectOnto,
  proportionalHandles,
  type ProportionalHandle,
} from './deltas'
import {
  planDrag,
  refKey,
  uniqueRefs,
  type DragContext,
  type DragPlan,
  type Modifiers,
} from './plan'
import {
  buildDragPreview,
  type DragPreview,
  type MirrorTarget,
} from './preview'

export {
  NO_MODIFIERS,
  planDrag,
  type DragContext,
  type DragLabels,
  type DragPlan,
  type Modifiers,
  type SnapResult,
} from './plan'
export { constrainDelta } from './deltas'
export type { DragPreview } from './preview'

/** A live drag. */
export class DragGesture {
  readonly plan: DragPlan
  readonly origin: Point2

  private readonly context: DragContext
  private readonly session: EditSession
  private readonly movedRefs: PointRef[]
  private delta: Point2 = { x: 0, y: 0 }
  private sent: Point2 = { x: 0, y: 0 }
  /** Per-point deltas, when the gesture constrains each point separately. */
  private deltas: Map<string, Point2> | null = null
  private sentDeltas = new Map<string, Point2>()
  /** Handles of neighbouring nodes that a proportional nudge rescales. */
  private proportional: ProportionalHandle[] = []
  private mirror: MirrorTarget | null = null
  private snapLabel: string | null = null

  constructor(
    context: DragContext,
    plan: DragPlan,
    origin: Point2,
    modifiers: Modifiers,
  ) {
    this.context = context
    this.plan = plan
    this.origin = origin
    this.session = new EditSession(context.doc, plan.label, {
      scheduler: context.scheduler,
      onError: context.onError,
      after: context.after,
    })
    this.movedRefs =
      plan.handleMode === 'WithHandles'
        ? uniqueRefs([
            ...plan.refs,
            ...plan.refs.flatMap((ref) => handlesOfNode(context.contours, ref)),
          ])
        : uniqueRefs(plan.refs)

    // Ctrl+Alt on a node nudges: the node and its own handles move, and the
    // handles of the neighbouring nodes keep their share of each segment
    // (design spec §4.3).
    if (plan.kind === 'points' && modifiers.mod && modifiers.alt)
      this.proportional = proportionalHandles(
        context.contours,
        plan.refs,
        new Set(this.movedRefs.map(refKey)),
      )

    // Ctrl+Alt on a smooth node's handle mirrors length and angle onto the
    // opposite handle (design spec §4.3).
    if (plan.kind === 'handle' && modifiers.mod && modifiers.alt) {
      const handle = plan.refs[0]
      const node = handle ? nodeOfHandle(context.contours, handle) : null
      const opposite = handle ? oppositeHandle(context.contours, handle) : null
      const nodePoint = node ? pointAt(context.contours, node) : null
      const oppositePoint = opposite
        ? pointAt(context.contours, opposite)
        : null
      if (opposite && nodePoint && oppositePoint)
        this.mirror = {
          handle: opposite,
          node: { x: nodePoint.x, y: nodePoint.y },
          from: { x: oppositePoint.x, y: oppositePoint.y },
        }
    }
  }

  /** Resolves once the kernel has answered `edit_begin`. */
  ready(): Promise<void> {
    return this.session.ready()
  }

  /** The delta the gesture currently shows, in font units. */
  get currentDelta(): Point2 {
    return this.delta
  }

  /** What the gesture snapped to, for the HUD. */
  get snappedTo(): string | null {
    return this.snapLabel
  }

  /** Whether anything has been sent to the kernel yet. */
  get dirty(): boolean {
    return this.session.dirty
  }

  /** Whether any kernel call of this gesture failed. */
  get failed(): boolean {
    return this.session.failed
  }

  /** Moves the gesture to a new pointer position, in font units. */
  move(pointer: Point2, modifiers: Modifiers): void {
    let delta = { x: pointer.x - this.origin.x, y: pointer.y - this.origin.y }
    if (modifiers.shift) delta = constrainDelta(delta)
    // Alt keeps a handle on its own line (design spec §4.3).
    let angleLocked = false
    if (this.plan.kind === 'handle' && modifiers.alt && !modifiers.mod) {
      const handle = this.plan.refs[0]
      const node = handle ? nodeOfHandle(this.context.contours, handle) : null
      const nodePoint = node ? pointAt(this.context.contours, node) : null
      const handlePoint = handle ? pointAt(this.context.contours, handle) : null
      if (nodePoint && handlePoint) {
        delta = projectOnto(delta, nodePoint, handlePoint)
        angleLocked = true
      }
    }
    // ⌥ on a segment reshapes it while every handle keeps its own angle
    // (design spec §4.3), which means one delta per control rather than one
    // for the gesture.
    this.deltas =
      this.plan.kind === 'segment' && modifiers.alt && !modifiers.mod
        ? perHandleDeltas(this.context.contours, this.plan.refs, delta)
        : this.proportional.length > 0
          ? nudgeDeltas(this.movedRefs, this.proportional, delta)
          : null
    if (this.deltas && this.plan.kind === 'segment') angleLocked = true

    this.snapLabel = null
    // Snapping would pull an angle-locked handle off its line, which is the
    // one thing Alt promised it would not do.
    if (!modifiers.mod && !angleLocked && this.context.snap) {
      const moved = this.movedPoints(delta)
      const snapped = this.context.snap(delta, moved)
      delta = snapped.delta
      this.snapLabel = snapped.label
    }
    if (delta.x === this.delta.x && delta.y === this.delta.y) return
    this.delta = delta
    this.session.queue(() => this.command())
  }

  /** The optimistic geometry the canvas should draw. */
  get preview(): DragPreview {
    return buildDragPreview({
      contours: this.context.contours,
      anchors: this.context.anchors,
      components: this.context.components,
      movedRefs: this.movedRefs,
      anchorIndexes: this.plan.anchors,
      componentIndexes: this.plan.components,
      delta: this.delta,
      deltas: this.deltas,
      mirror: this.mirror,
      mirrorPoint: this.mirror ? this.mirrorTarget() : null,
    })
  }

  /** Sends whatever is queued right now; used by tests and by pointer-up. */
  flush(): Promise<void> {
    return this.session.flush()
  }

  /**
   * Ends the gesture. Resolves to `true` when the kernel recorded an undo
   * entry, which a gesture that never moved does not.
   */
  async finish(): Promise<boolean> {
    await this.session.end()
    return this.session.dirty
  }

  /** Escape during a drag: closes the gesture and undoes what it did. */
  cancel(): Promise<void> {
    return this.session.cancel()
  }

  private movedPoints(delta: Point2): Point2[] {
    const points: Point2[] = []
    for (const ref of this.plan.refs) {
      const point = pointAt(this.context.contours, ref)
      if (point) points.push({ x: point.x + delta.x, y: point.y + delta.y })
    }
    for (const index of this.plan.anchors) {
      const anchor = this.context.anchors[index]
      if (anchor) points.push({ x: anchor.x + delta.x, y: anchor.y + delta.y })
    }
    return points
  }

  private mirrorTarget(): Point2 {
    const mirror = this.mirror
    if (!mirror) return { x: 0, y: 0 }
    const handle = this.plan.refs[0]
    const start = handle ? pointAt(this.context.contours, handle) : null
    if (!start) return mirror.from
    const moved = {
      x: start.x + this.delta.x,
      y: start.y + this.delta.y,
    }
    // Mirror length and angle through the node.
    return {
      x: 2 * mirror.node.x - moved.x,
      y: 2 * mirror.node.y - moved.y,
    }
  }

  /**
   * The command for the current position.
   *
   * `MovePoints` and `TransformSelection` translate the stored state, so what
   * goes on the wire is the delta *since the previous apply*; the sum over the
   * gesture is the delta the user dragged.
   */
  private command(): Command | null {
    const target = { glyph: this.context.glyph, layer: this.context.layer }
    if (this.deltas)
      return perHandleCommand(target, this.deltas, this.sentDeltas)
    const step = {
      x: this.delta.x - this.sent.x,
      y: this.delta.y - this.sent.y,
    }
    if (step.x === 0 && step.y === 0 && !this.mirror) return null
    this.sent = { ...this.delta }
    return stepCommand(target, {
      refs: this.plan.refs,
      anchors: this.plan.anchors,
      components: this.plan.components,
      handles: this.plan.handleMode,
      step,
      mirror: this.mirror
        ? { point: this.mirror.handle, at: this.mirrorTarget() }
        : null,
    })
  }
}

/** Starts a drag, or returns `null` when the hit is not draggable. */
export function beginDrag(
  context: DragContext,
  hit: Hit,
  origin: Point2,
  modifiers: Modifiers,
): DragGesture | null {
  const plan = planDrag(context, hit, modifiers)
  if (!plan) return null
  if (
    plan.refs.length === 0 &&
    plan.anchors.length === 0 &&
    plan.components.length === 0
  )
    return null
  return new DragGesture(context, plan, origin, modifiers)
}
