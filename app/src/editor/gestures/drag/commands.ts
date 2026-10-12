/**
 * What a drag puts on the wire.
 *
 * `MovePoints` and `TransformSelection` translate the *current* stored state,
 * so every apply carries the step since the previous one; their sum over the
 * gesture is the delta the user dragged, in one coalesced undo entry.
 */

import type { Command, HandleMode, PointRef } from '../../../api'
import type { Point2 } from '../../../canvas/Viewport'

/** The glyph and layer every drag command names. */
export interface CommandTarget {
  glyph: string
  layer: string
}

/** One command, or a `Batch` when the step needs several. */
function combine(commands: readonly Command[]): Command | null {
  if (commands.length === 0) return null
  return commands.length === 1
    ? (commands[0] as Command)
    : { Batch: [...commands] }
}

/**
 * One `MovePoints` per individually constrained point, each with the step
 * since its last apply. `sent` is updated in place, so the caller keeps one
 * record of what the kernel already has.
 */
export function perHandleCommand(
  target: CommandTarget,
  deltas: ReadonlyMap<string, Point2>,
  sent: Map<string, Point2>,
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
    const already = sent.get(key) ?? { x: 0, y: 0 }
    const step = { x: total.x - already.x, y: total.y - already.y }
    if (step.x === 0 && step.y === 0) continue
    sent.set(key, { ...total })
    commands.push({
      MovePoints: {
        glyph: target.glyph,
        layer: target.layer,
        points: [ref],
        delta: [step.x, step.y],
        handles: 'PointsOnly',
      },
    })
  }
  return combine(commands)
}

/** What one uniform-delta step sends. */
export interface StepInput {
  refs: readonly PointRef[]
  anchors: readonly number[]
  components: readonly number[]
  handles: HandleMode
  step: Point2
  /** Absolute position of a mirrored handle, when the gesture has one. */
  mirror: { point: PointRef; at: Point2 } | null
}

/** The commands for one uniform-delta step of a drag. */
export function stepCommand(
  target: CommandTarget,
  input: StepInput,
): Command | null {
  const commands: Command[] = []
  if (input.refs.length > 0)
    commands.push({
      MovePoints: {
        glyph: target.glyph,
        layer: target.layer,
        points: [...input.refs],
        delta: [input.step.x, input.step.y],
        handles: input.handles,
      },
    })
  if (input.anchors.length > 0 || input.components.length > 0)
    commands.push({
      TransformSelection: {
        glyph: target.glyph,
        layer: target.layer,
        points: [],
        components: [...input.components],
        anchors: [...input.anchors],
        affine: [1, 0, 0, 1, input.step.x, input.step.y],
      },
    })
  if (input.mirror)
    commands.push({
      SetPointPosition: {
        glyph: target.glyph,
        layer: target.layer,
        point: input.mirror.point,
        x: input.mirror.at.x,
        y: input.mirror.at.y,
      },
    })
  return combine(commands)
}
