/**
 * The pointer grammar of a smart-component handle on the glyph canvas.
 *
 * A press on a handle takes the pointer for the whole drag, the handle follows
 * it with its value shown beside it, and the release writes one `SetComponent`
 * — so one drag is one undo entry and every other axis and the component's own
 * transform are preserved. Escape or leaving the canvas abandons the drag.
 */

import type { ComponentView } from '../../../api'
import type { Point2, Viewport } from '../../../canvas/Viewport'
import {
  setComponentCommand,
  type LayerTarget,
} from '../../../editor/pointCommands'
import type { Selection } from '../../../editor/selection'
import { i18n } from '../../../i18n'
import { commitField } from '../commitField'

import {
  smartHandleAt,
  smartHandles,
  smartValueFor,
  type SmartHandle,
} from './smartComponentCanvas'
import { canonicalSmart } from './smartComponentValues'

/** A canvas pointer event, in the shape the glyph canvas already builds. */
export interface SmartHandleInput {
  x: number
  y: number
  button: number
}

export class SmartComponentPointer {
  private gesture: SmartHandle | null = null
  private value: number | null = null
  private hovered: SmartHandle | null = null

  /** Whether a drag is in progress. */
  get busy(): boolean {
    return this.gesture !== null
  }

  /** The handle to emphasise, with its in-progress value. */
  active(): SmartHandle | null {
    if (!this.gesture) return this.hovered
    return this.value === null
      ? this.gesture
      : { ...this.gesture, ...this.positioned(this.gesture, this.value) }
  }

  /** The handles the canvas should draw, including the dragged one. */
  handles(
    components: readonly ComponentView[],
    selection: Selection,
    viewport: Viewport,
    glyphLocked: boolean,
  ): SmartHandle[] {
    const handles = smartHandles(components, selection, viewport, glyphLocked)
    const gesture = this.gesture
    if (!gesture || this.value === null) return handles
    return handles.map((handle) =>
      handle.component === gesture.component && handle.axis === gesture.axis
        ? { ...handle, ...this.positioned(handle, this.value as number) }
        : handle,
    )
  }

  private positioned(
    handle: SmartHandle,
    value: number,
  ): { value: number; at: Point2 } {
    const span = handle.max - handle.min
    const share =
      Math.abs(span) < 1e-12
        ? 0
        : Math.max(0, Math.min(1, (value - handle.min) / span))
    return {
      value,
      at: {
        x: handle.from.x + (handle.to.x - handle.from.x) * share,
        y: handle.from.y + (handle.to.y - handle.from.y) * share,
      },
    }
  }

  /** Starts a drag; `false` leaves the press to the outline editor. */
  down(
    components: readonly ComponentView[],
    selection: Selection,
    viewport: Viewport,
    glyphLocked: boolean,
    input: SmartHandleInput,
  ): boolean {
    this.gesture = null
    this.value = null
    if (input.button !== 0) return false
    const handles = smartHandles(components, selection, viewport, glyphLocked)
    const handle = smartHandleAt(handles, viewport, { x: input.x, y: input.y })
    if (!handle) return false
    this.gesture = handle
    this.value = handle.value
    return true
  }

  /** Tracks a drag, or records the hovered handle when idle. */
  move(
    components: readonly ComponentView[],
    selection: Selection,
    viewport: Viewport,
    glyphLocked: boolean,
    input: SmartHandleInput,
  ): boolean {
    const at = { x: input.x, y: input.y }
    if (!this.gesture) {
      const handles = smartHandles(components, selection, viewport, glyphLocked)
      const next = smartHandleAt(handles, viewport, at)
      const changed = next !== this.hovered
      this.hovered = next
      return changed
    }
    this.value = smartValueFor(this.gesture, viewport.toFont(at))
    return true
  }

  /**
   * Commits a finished drag as one `SetComponent`; `false` leaves the release
   * to the outline editor.
   *
   * A drag that ends on the value it started from writes nothing.
   */
  up(
    components: readonly ComponentView[],
    target: LayerTarget | null,
    onReload: () => void,
  ): boolean {
    const gesture = this.gesture
    const value = this.value
    this.gesture = null
    this.value = null
    if (!gesture) return false
    const view = components[gesture.component]
    if (!target || !view || value === null || value === gesture.value)
      return true
    const axes = view.smart_axes ?? []
    const axis = axes[gesture.axis]
    if (!axis || axis.name !== gesture.name) return true
    const smart = canonicalSmart(axes, {
      ...(view.component.smart ?? {}),
      [axis.name]: value,
    })
    commitField(
      target,
      setComponentCommand(target, gesture.component, {
        ...view.component,
        smart: Object.keys(smart).length > 0 ? smart : null,
      }),
      i18n.t('panel.smartComponent.undo'),
      onReload,
    )
    return true
  }

  /** Abandons an open drag without writing it. */
  cancel(): void {
    this.gesture = null
    this.value = null
    this.hovered = null
  }
}
