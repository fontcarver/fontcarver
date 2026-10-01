/** Which key presses step the proof library, in the field and on the canvas. */

/** The keyboard fields the predicate reads; DOM and React events both conform. */
export interface ProofStepEvent {
  key: string
  altKey: boolean
  ctrlKey: boolean
  metaKey: boolean
  shiftKey: boolean
  isComposing?: boolean
  defaultPrevented?: boolean
}

/**
 * `-1` for the previous entry, `1` for the next, `null` for anything else.
 *
 * Only a bare Up or Down steps. Every modifier is reserved — the shell's own
 * bindings and the editor's ×10 and ×100 nudges use them — and a key being
 * composed belongs to the input method.
 */
export function proofStepDirection(event: ProofStepEvent): -1 | 1 | null {
  if (
    event.isComposing ||
    event.defaultPrevented ||
    event.altKey ||
    event.ctrlKey ||
    event.metaKey ||
    event.shiftKey
  )
    return null
  if (event.key === 'ArrowUp') return -1
  if (event.key === 'ArrowDown') return 1
  return null
}

/** What the surface-wide listener needs to know before it claims a key. */
export interface ProofStepContext {
  /** True while the outline editor holds a node selection to nudge. */
  hasSelection: boolean
  /** True when the key came from a text field, which has its own listener. */
  editableTarget: boolean
  /**
   * True when the key came from a focusable control that owns the arrow keys
   * itself — the preview strip's resize separator, a slider, a tab strip. The
   * glyph canvas is focusable but is deliberately not one of these.
   */
  interactiveTarget?: boolean
}

/**
 * Whether the text view may step the library for this press.
 *
 * Spacing happens with the canvas focused and nothing selected, which is where
 * the row's Up/Down gesture belongs. A selected node still nudges, a text field
 * keeps its own handler, and a focusable control that moves on Up and Down
 * keeps its own keys, so no two paths ever both fire.
 */
export function canStepFromSurface(context: ProofStepContext): boolean {
  return (
    !context.hasSelection &&
    !context.editableTarget &&
    !context.interactiveTarget
  )
}
