/**
 * The Hand tool (design spec §5): pan the view.
 *
 * Hand is reachable three ways — the tool, `H`, and Space held down — plus the
 * middle mouse button, which pans whatever tool is active. All of them end up
 * here so panning behaves identically however it was started.
 */

import type { Viewport } from '../../canvas/Viewport'

/** Cursor while the hand is idle and while it is dragging. */
export const HAND_CURSOR = 'grab'
export const HAND_DRAGGING_CURSOR = 'grabbing'

/** Pans the viewport by a canvas-space delta. */
export function pan(viewport: Viewport, dx: number, dy: number): void {
  viewport.pan(dx, dy)
}
