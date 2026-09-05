/**
 * The Hand tool (design spec §5): pan the view.
 *
 * Hand is reachable three ways — the tool, `H`, and Space held down — plus the
 * middle mouse button, which pans whatever tool is active. All four end in the
 * controller's pan branch, so what this module owns is the cursor that says
 * which of the two states the hand is in.
 */

/** Cursor while the hand is idle and while it is dragging. */
export const HAND_CURSOR = 'grab'
export const HAND_DRAGGING_CURSOR = 'grabbing'
