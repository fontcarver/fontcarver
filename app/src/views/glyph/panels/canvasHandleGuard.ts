/**
 * When the canvas offers its own handles.
 *
 * The smart-component axis tracks and the source-image placement handles sit
 * above the outline and take the pointer before the editor sees it. The view
 * gestures must still reach the canvas: a Hand pan, a Zoom click and a
 * space-held pan are the ways out of any tool, so a press that starts on a
 * handle while one of them is active belongs to the view, not to the handle.
 * The same answer hides the handles, so nothing is drawn that cannot be used.
 */

import type { ToolId } from '../../../editor/tools/registry'

/** Tools whose press is a view gesture rather than an edit. */
const VIEW_TOOLS: readonly ToolId[] = ['hand', 'zoom']

/** Whether canvas handles may be drawn and may take the pointer. */
export function canvasHandlesActive(tool: ToolId, spaceHeld: boolean): boolean {
  return !spaceHeld && !VIEW_TOOLS.includes(tool)
}
