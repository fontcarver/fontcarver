/**
 * The Rotate tool (design spec §5): a click sets the origin, a drag turns the
 * selection about it, and Shift constrains the turn to 15° steps.
 */

import type { Point2 } from '../../canvas/Viewport'

import type { CursorId } from '../cursors'
import type { Modifiers } from '../gestures/drag'
import type { TransformAmount } from '../gestures/transform'

import type { ToolId } from './registry'
import type { ToolContext } from './toolTypes'
import { ROTATE_STEP_DEGREES, TransformTool } from './transformTool'

export class RotateTool extends TransformTool {
  readonly id: ToolId = 'rotate'
  protected readonly kind = 'rotate' as const
  protected readonly cursorId: CursorId = 'rotate'

  protected amountFor(
    origin: Point2,
    from: Point2,
    to: Point2,
    modifiers: Modifiers,
  ): TransformAmount {
    const start = Math.atan2(from.y - origin.y, from.x - origin.x)
    const now = Math.atan2(to.y - origin.y, to.x - origin.x)
    let radians = now - start
    if (modifiers.shift) {
      const step = (ROTATE_STEP_DEGREES * Math.PI) / 180
      radians = Math.round(radians / step) * step
    }
    return { kind: 'rotate', radians }
  }

  protected hudText(amount: TransformAmount, context: ToolContext): string {
    const degrees =
      amount.kind === 'rotate' ? (amount.radians * 180) / Math.PI : 0
    return `${context.labels.hud.angle} ${degrees.toFixed(1)}°`
  }
}
