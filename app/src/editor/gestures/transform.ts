/**
 * The rotate and scale gestures.
 *
 * Both follow exactly the protocol a drag does — one `edit_begin`, one
 * `edit_apply` per animation frame, one `edit_end`, which is one undo entry —
 * and both send `TransformSelection`, which transforms the *current* stored
 * state. What goes on the wire is therefore the step since the previous apply,
 * while the preview always shows the total transform applied to the geometry
 * the gesture started from.
 *
 * With nothing selected the command carries no points, no anchors and no
 * components, which is how the kernel is told to transform the whole layer.
 */

import type {
  Anchor,
  Command,
  ComponentView,
  Contour,
  DocId,
  PointRef,
} from '../../api'
import type { Point2 } from '../../canvas/Viewport'
import {
  applyAffine,
  composeAffine,
  rotateAbout,
  scaleAbout,
  transformBox,
  IDENTITY,
  type Affine,
} from '../affine'
import { EditSession, type Scheduler } from '../editSession'
import type { EditorPreview } from '../editorTypes'

/** How far a transform gesture has been taken. */
export type TransformAmount =
  | { kind: 'rotate'; radians: number }
  | { kind: 'scale'; sx: number; sy: number }

/** Everything a transform gesture needs about the layer it edits. */
export interface TransformContext {
  doc: DocId
  glyph: string
  layer: string
  contours: readonly Contour[]
  anchors: readonly Anchor[]
  components: readonly ComponentView[]
  /** Stored points to transform; empty means the whole layer. */
  points: readonly PointRef[]
  anchorIndexes: readonly number[]
  componentIndexes: readonly number[]
  /** The fixed point of the transform, in font units. */
  origin: Point2
  /** Undo label, already translated. */
  label: string
  scheduler?: Scheduler
  onError?: (error: unknown) => void
  after?: Promise<unknown>
}

/** The identity amount for a gesture of the given kind. */
function neutral(kind: TransformAmount['kind']): TransformAmount {
  return kind === 'rotate'
    ? { kind: 'rotate', radians: 0 }
    : { kind: 'scale', sx: 1, sy: 1 }
}

/** The affine one amount describes about `origin`. */
function affineOf(amount: TransformAmount, origin: Point2): Affine {
  return amount.kind === 'rotate'
    ? rotateAbout(origin, amount.radians)
    : scaleAbout(origin, amount.sx, amount.sy)
}

/** The affine that takes `from` to `to`, both measured from the same origin. */
function stepBetween(
  from: TransformAmount,
  to: TransformAmount,
  origin: Point2,
): Affine {
  if (to.kind === 'rotate' && from.kind === 'rotate')
    return rotateAbout(origin, to.radians - from.radians)
  if (to.kind === 'scale' && from.kind === 'scale')
    return scaleAbout(
      origin,
      from.sx === 0 ? to.sx : to.sx / from.sx,
      from.sy === 0 ? to.sy : to.sy / from.sy,
    )
  return affineOf(to, origin)
}

/** Whether an affine is close enough to the identity to send nothing. */
function isIdentity(affine: Affine): boolean {
  return affine.every(
    (value, index) => Math.abs(value - IDENTITY[index]) < 1e-12,
  )
}

/** A live rotate or scale. */
export class TransformGesture {
  private readonly context: TransformContext
  private readonly session: EditSession
  private readonly wholeLayer: boolean
  private readonly moved: ReadonlySet<string>
  private amount: TransformAmount
  private sent: TransformAmount

  constructor(context: TransformContext, kind: TransformAmount['kind']) {
    this.context = context
    this.amount = neutral(kind)
    this.sent = neutral(kind)
    this.wholeLayer =
      context.points.length === 0 &&
      context.anchorIndexes.length === 0 &&
      context.componentIndexes.length === 0
    this.moved = new Set(
      context.points.map((ref) => `${ref.contour}:${ref.point}`),
    )
    this.session = new EditSession(context.doc, context.label, {
      scheduler: context.scheduler,
      onError: context.onError,
      after: context.after,
    })
  }

  /** Resolves once the kernel has answered `edit_begin`. */
  ready(): Promise<void> {
    return this.session.ready()
  }

  /** Whether anything reached the kernel, i.e. an undo entry exists. */
  get dirty(): boolean {
    return this.session.dirty
  }

  /** Whether any call of this gesture failed. */
  get failed(): boolean {
    return this.session.failed
  }

  /** How far the gesture has been taken. */
  get current(): TransformAmount {
    return this.amount
  }

  /** Moves the gesture to a new total amount. */
  update(amount: TransformAmount): void {
    if (sameAmount(amount, this.amount)) return
    this.amount = amount
    this.session.queue(() => this.command())
  }

  /** The optimistic geometry the canvas should draw. */
  get preview(): EditorPreview {
    const affine = affineOf(this.amount, this.context.origin)
    const { contours, anchors, components } = this.context
    return {
      contours: contours.map((contour, contourIndex) => ({
        ...contour,
        points: contour.points.map((point, index) => {
          if (!this.wholeLayer && !this.moved.has(`${contourIndex}:${index}`))
            return point
          const moved = applyAffine(affine, point)
          return { ...point, x: moved.x, y: moved.y }
        }),
      })),
      anchors: anchors.map((anchor, index) => {
        if (!this.wholeLayer && !this.context.anchorIndexes.includes(index))
          return anchor
        const moved = applyAffine(affine, anchor)
        return { ...anchor, x: moved.x, y: moved.y }
      }),
      components: components.map((view, index) => {
        if (!this.wholeLayer && !this.context.componentIndexes.includes(index))
          return view
        const transform = composeAffine(affine, view.component.transform)
        const box = view.bbox
          ? transformBox(affine, {
              x0: view.bbox[0],
              y0: view.bbox[1],
              x1: view.bbox[2],
              y1: view.bbox[3],
            })
          : null
        return {
          ...view,
          component: { ...view.component, transform },
          bbox: box ? [box.x0, box.y0, box.x1, box.y1] : view.bbox,
        }
      }),
    }
  }

  /** Sends whatever is queued right now. */
  flush(): Promise<void> {
    return this.session.flush()
  }

  /** Ends the gesture; resolves `true` when an undo entry was recorded. */
  async finish(): Promise<boolean> {
    await this.session.end()
    return this.session.dirty
  }

  /** Escape: closes the gesture and undoes whatever it applied. */
  cancel(): Promise<void> {
    return this.session.cancel()
  }

  private command(): Command | null {
    const step = stepBetween(this.sent, this.amount, this.context.origin)
    if (isIdentity(step)) return null
    this.sent = this.amount
    return {
      TransformSelection: {
        glyph: this.context.glyph,
        layer: this.context.layer,
        points: [...this.context.points],
        components: [...this.context.componentIndexes],
        anchors: [...this.context.anchorIndexes],
        affine: step,
      },
    }
  }
}

function sameAmount(a: TransformAmount, b: TransformAmount): boolean {
  if (a.kind === 'rotate' && b.kind === 'rotate') return a.radians === b.radians
  if (a.kind === 'scale' && b.kind === 'scale')
    return a.sx === b.sx && a.sy === b.sy
  return false
}
