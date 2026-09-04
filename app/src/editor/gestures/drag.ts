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
import { EditSession, frameScheduler, type Scheduler } from '../editSession'
import type { Hit } from '../hitTest'
import {
  buildEditOutline,
  handlesOfNode,
  nodeOfHandle,
  oppositeHandle,
  pointAt,
  setPointPosition,
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
      handleMode: modifiers.alt ? 'PointsOnly' : 'WithHandles',
      label: labels.movePoints(Math.max(nodes, refs.length ? nodes : 1)),
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
    this.session = new EditSession(
      context.doc,
      plan.label,
      context.scheduler ?? frameScheduler,
    )
    this.movedRefs =
      plan.handleMode === 'WithHandles'
        ? uniqueRefs([
            ...plan.refs,
            ...plan.refs.flatMap((ref) => handlesOfNode(context.contours, ref)),
          ])
        : uniqueRefs(plan.refs)

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

  /** Moves the gesture to a new pointer position, in font units. */
  move(pointer: Point2, modifiers: Modifiers): void {
    let delta = { x: pointer.x - this.origin.x, y: pointer.y - this.origin.y }
    if (modifiers.shift) delta = this.constrain(delta)
    // Alt keeps a handle on its own line (design spec §4.3).
    if (this.plan.kind === 'handle' && modifiers.alt && !modifiers.mod) {
      const handle = this.plan.refs[0]
      const node = handle ? nodeOfHandle(this.context.contours, handle) : null
      const nodePoint = node ? pointAt(this.context.contours, node) : null
      const handlePoint = handle ? pointAt(this.context.contours, handle) : null
      if (nodePoint && handlePoint)
        delta = projectOnto(delta, nodePoint, handlePoint)
    }
    this.snapLabel = null
    if (!modifiers.mod && this.context.snap) {
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
    const { contours, anchors } = this.context
    const moved = translatePoints(
      contours,
      this.movedRefs,
      this.delta.x,
      this.delta.y,
    )
    const withMirror = this.mirror
      ? setPointPosition(
          moved,
          this.mirror.handle,
          this.mirrorTarget().x,
          this.mirrorTarget().y,
        )
      : moved
    const movedAnchors =
      this.plan.anchors.length === 0
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
