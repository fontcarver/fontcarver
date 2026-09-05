/**
 * Dragging a selection with the Select tool.
 *
 * The gesture owns three things: what moves (planned once, on pointer-down),
 * where it moves to (recomputed from the gesture origin on every pointer move,
 * after the modifier constraints and snapping have had their say) and the
 * kernel protocol (one `edit_begin`, throttled `edit_apply`s, one `edit_end`,
 * which is one undo entry).
 *
 * The canvas draws from `preview`, a local copy of the outline that is updated
 * synchronously on every pointer move, so the outline never lags the pointer
 * even though the kernel is only told once per animation frame.
 */

import type {
  Anchor,
  Command,
  Contour,
  DocId,
  HandleMode,
  PointRef,
} from '../../api'
import type { Point2 } from '../../canvas/Viewport'
import { EditSession, type Scheduler } from '../editSession'
import type { Hit } from '../hitTest'
import {
  buildEditOutline,
  handlesOfNode,
  nodeOfHandle,
  oppositeHandle,
  pointAt,
  setPointPosition,
  translateEach,
  translatePoints,
} from '../outlineGeometry'
import {
  isSelected,
  selectedIndexes,
  selectedNodeRefs,
  selectedPointRefs,
  type Selection,
} from '../selection'

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

function refKey(ref: PointRef): string {
  return `${ref.contour}:${ref.point}`
}

function uniqueRefs(refs: readonly PointRef[]): PointRef[] {
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

/** Constrains a delta to the nearest 0/45/90° direction. */
export function constrainDelta(delta: Point2): Point2 {
  const angle = Math.atan2(delta.y, delta.x)
  const step = Math.PI / 4
  const snapped = Math.round(angle / step) * step
  const length = Math.hypot(delta.x, delta.y)
  const x = Math.cos(snapped) * length
  const y = Math.sin(snapped) * length
  // Zero the axis that the constraint collapsed, so an axis-locked drag is
  // exactly axis-locked rather than a rounding error away from it.
  return {
    x: Math.abs(x) < 1e-9 ? 0 : x,
    y: Math.abs(y) < 1e-9 ? 0 : y,
  }
}

/** Projects a delta onto the direction from `from` to `to`. */
function projectOnto(delta: Point2, from: Point2, to: Point2): Point2 {
  const dx = to.x - from.x
  const dy = to.y - from.y
  const length = Math.hypot(dx, dy)
  if (length < 1e-9) return delta
  const unit = { x: dx / length, y: dy / length }
  const amount = delta.x * unit.x + delta.y * unit.y
  return { x: unit.x * amount, y: unit.y * amount }
}

/** The optimistic local geometry a drag is showing. */
export interface DragPreview {
  contours: readonly Contour[]
  anchors: readonly Anchor[]
}

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
  private mirror: { handle: PointRef; node: Point2; from: Point2 } | null = null
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
        new Set(this.movedRefs.map((ref) => `${ref.contour}:${ref.point}`)),
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
    if (modifiers.shift) delta = this.constrain(delta)
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
        ? this.perHandleDeltas(delta)
        : this.proportional.length > 0
          ? this.nudgeDeltas(delta)
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

  /**
   * The deltas that keep every control of a segment on its own handle line.
   */
  private perHandleDeltas(delta: Point2): Map<string, Point2> | null {
    const deltas = new Map<string, Point2>()
    for (const ref of this.plan.refs) {
      const handle = pointAt(this.context.contours, ref)
      const node = nodeOfHandle(this.context.contours, ref)
      const nodePoint = node ? pointAt(this.context.contours, node) : null
      if (!handle || !nodePoint) return null
      deltas.set(
        `${ref.contour}:${ref.point}`,
        projectOnto(delta, nodePoint, handle),
      )
    }
    return deltas.size > 0 ? deltas : null
  }

  /**
   * A proportional nudge: everything the gesture carries moves by `delta`, and
   * each neighbouring node's handle keeps its share of the segment whose chord
   * the move just stretched or shortened.
   */
  private nudgeDeltas(delta: Point2): Map<string, Point2> {
    const deltas = new Map<string, Point2>()
    for (const ref of this.movedRefs)
      deltas.set(`${ref.contour}:${ref.point}`, delta)
    for (const entry of this.proportional) {
      const before = Math.hypot(
        entry.node.x - entry.anchor.x,
        entry.node.y - entry.anchor.y,
      )
      if (before < 1e-9) continue
      const after = Math.hypot(
        entry.node.x + delta.x - entry.anchor.x,
        entry.node.y + delta.y - entry.anchor.y,
      )
      const scale = after / before
      deltas.set(`${entry.ref.contour}:${entry.ref.point}`, {
        x:
          entry.anchor.x +
          (entry.handle.x - entry.anchor.x) * scale -
          entry.handle.x,
        y:
          entry.anchor.y +
          (entry.handle.y - entry.anchor.y) * scale -
          entry.handle.y,
      })
    }
    return deltas
  }

  /** The optimistic geometry the canvas should draw. */
  get preview(): DragPreview {
    const { contours, anchors } = this.context
    const moved = this.deltas
      ? translateEach(contours, this.deltas)
      : translatePoints(contours, this.movedRefs, this.delta.x, this.delta.y)
    const withMirror = this.mirror
      ? setPointPosition(
          moved,
          this.mirror.handle,
          this.mirrorTarget().x,
          this.mirrorTarget().y,
        )
      : moved
    // Anchors and components ride the gesture's single `TransformSelection`,
    // which a per-point regime does not send: previewing them moving would
    // promise something the gesture never applies, and they would snap back on
    // the reload that follows it.
    const movedAnchors =
      this.deltas !== null || this.plan.anchors.length === 0
        ? anchors
        : anchors.map((anchor, index) =>
            this.plan.anchors.includes(index)
              ? {
                  ...anchor,
                  x: anchor.x + this.delta.x,
                  y: anchor.y + this.delta.y,
                }
              : anchor,
          )
    return { contours: withMirror, anchors: movedAnchors }
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

  private constrain(delta: Point2): Point2 {
    return constrainDelta(delta)
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

  /** One `MovePoints` per control, each with the step since its last apply. */
  private perHandleCommand(
    deltas: ReadonlyMap<string, Point2>,
  ): Command | null {
    const commands: Command[] = []
    // Every point the gesture constrains individually, in stored order so the
    // batch is deterministic — the map also carries points that are not in
    // `plan.refs`, such as a neighbour's handle under a proportional nudge.
    const keys = [...deltas.keys()].sort((a, b) => {
      const [ac = '0', ap = '0'] = a.split(':')
      const [bc = '0', bp = '0'] = b.split(':')
      return Number(ac) - Number(bc) || Number(ap) - Number(bp)
    })
    for (const key of keys) {
      const [contour = '0', point = '0'] = key.split(':')
      const ref: PointRef = { contour: Number(contour), point: Number(point) }
      const total = deltas.get(key)
      if (!total) continue
      const sent = this.sentDeltas.get(key) ?? { x: 0, y: 0 }
      const step = { x: total.x - sent.x, y: total.y - sent.y }
      if (step.x === 0 && step.y === 0) continue
      this.sentDeltas.set(key, { ...total })
      commands.push({
        MovePoints: {
          glyph: this.context.glyph,
          layer: this.context.layer,
          points: [ref],
          delta: [step.x, step.y],
          handles: 'PointsOnly',
        },
      })
    }
    if (commands.length === 0) return null
    return commands.length === 1
      ? (commands[0] as Command)
      : { Batch: commands }
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
    if (this.deltas) return this.perHandleCommand(this.deltas)
    const step = {
      x: this.delta.x - this.sent.x,
      y: this.delta.y - this.sent.y,
    }
    if (step.x === 0 && step.y === 0 && !this.mirror) return null
    this.sent = { ...this.delta }
    const commands: Command[] = []
    if (this.plan.refs.length > 0)
      commands.push({
        MovePoints: {
          glyph: this.context.glyph,
          layer: this.context.layer,
          points: this.plan.refs,
          delta: [step.x, step.y],
          handles: this.plan.handleMode,
        },
      })
    if (this.plan.anchors.length > 0 || this.plan.components.length > 0)
      commands.push({
        TransformSelection: {
          glyph: this.context.glyph,
          layer: this.context.layer,
          points: [],
          components: this.plan.components,
          anchors: this.plan.anchors,
          affine: [1, 0, 0, 1, step.x, step.y],
        },
      })
    if (this.mirror) {
      const target = this.mirrorTarget()
      commands.push({
        SetPointPosition: {
          glyph: this.context.glyph,
          layer: this.context.layer,
          point: this.mirror.handle,
          x: target.x,
          y: target.y,
        },
      })
    }
    if (commands.length === 0) return null
    return commands.length === 1
      ? (commands[0] as Command)
      : { Batch: commands }
  }
}

/** A neighbouring node's handle that a proportional nudge rescales. */
interface ProportionalHandle {
  ref: PointRef
  /** The handle's current position. */
  handle: Point2
  /** The node the handle belongs to, which stays put. */
  anchor: Point2
  /** The node being dragged, whose move changes the chord. */
  node: Point2
}

/**
 * The handles that a proportional nudge rescales.
 *
 * For every segment that ends at a dragged node, the control belonging to the
 * *other* end is collected — unless it is itself part of the gesture, in which
 * case it already travels with the selection and must not be scaled on top.
 */
function proportionalHandles(
  contours: readonly Contour[],
  refs: readonly PointRef[],
  moving: ReadonlySet<string>,
): ProportionalHandle[] {
  const outline = buildEditOutline(contours)
  const dragged = new Set(refs.map((ref) => `${ref.contour}:${ref.point}`))
  const found = new Map<string, ProportionalHandle>()
  for (const segment of outline.segments) {
    if (segment.controlRefs.length === 0) continue
    const ends: [PointRef | null, PointRef | null][] = [
      [segment.fromRef, segment.toRef],
      [segment.toRef, segment.fromRef],
    ]
    for (const [near, far] of ends) {
      if (!near || !far) continue
      if (!dragged.has(`${near.contour}:${near.point}`)) continue
      if (dragged.has(`${far.contour}:${far.point}`)) continue
      // The control adjacent to `far` is the one that belongs to it.
      const ref =
        far === segment.fromRef
          ? segment.controlRefs[0]
          : segment.controlRefs[segment.controlRefs.length - 1]
      if (!ref) continue
      const key = `${ref.contour}:${ref.point}`
      if (moving.has(key) || found.has(key)) continue
      const handle = pointAt(contours, ref)
      const anchor = pointAt(contours, far)
      const node = pointAt(contours, near)
      if (!handle || !anchor || !node) continue
      found.set(key, {
        ref,
        handle: { x: handle.x, y: handle.y },
        anchor: { x: anchor.x, y: anchor.y },
        node: { x: node.x, y: node.y },
      })
    }
  }
  return [...found.values()]
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
