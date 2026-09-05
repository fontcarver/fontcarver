/**
 * The Scale tool (design spec §5): a click sets the origin, a drag scales the
 * selection about it, and Shift keeps the two axes equal.
 */

import type { Point2 } from '../../canvas/Viewport'

import type { CursorId } from '../cursors'
import type { Modifiers } from '../gestures/drag'
import type { TransformAmount } from '../gestures/transform'

import type { ToolId } from './registry'
import type { ToolContext } from './toolTypes'
import { TransformTool } from './transformTool'

/** Below this distance from the origin an axis cannot be scaled sensibly. */
const MIN_REACH = 1e-6

export class ScaleTool extends TransformTool {
  readonly id: ToolId = 'scale'
  protected readonly kind = 'scale' as const
  protected readonly cursorId: CursorId = 'scale'

  protected amountFor(
    origin: Point2,
    from: Point2,
    to: Point2,
    modifiers: Modifiers,
  ): TransformAmount {
    if (modifiers.shift) {
      const before = Math.hypot(from.x - origin.x, from.y - origin.y)
      const after = Math.hypot(to.x - origin.x, to.y - origin.y)
      const factor = before < MIN_REACH ? 1 : after / before
      return { kind: 'scale', sx: factor, sy: factor }
    }
    const dx = from.x - origin.x
    const dy = from.y - origin.y
    return {
      kind: 'scale',
      sx: Math.abs(dx) < MIN_REACH ? 1 : (to.x - origin.x) / dx,
      sy: Math.abs(dy) < MIN_REACH ? 1 : (to.y - origin.y) / dy,
    }
  }

  protected hudText(amount: TransformAmount, context: ToolContext): string {
    if (amount.kind !== 'scale') return ''
    const percent = (value: number) => `${(value * 100).toFixed(1)}%`
    return `${context.labels.percent} ${percent(amount.sx)} × ${percent(amount.sy)}`
  }
}
