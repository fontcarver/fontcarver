/**
 * The Select tool's decisions (design spec §4.3, §5).
 *
 * Click grammar and hover cursors are pure functions of the hit and the
 * modifiers, so "what will this click do?" is answerable without a canvas —
 * which is exactly the promise the cursor makes to the user.
 */

import { toolCursor } from '../cursors'
import type { Modifiers } from '../gestures/drag'
import type { Hit } from '../hitTest'
import {
  extendSelection,
  isSelected,
  selectOne,
  toggleSelection,
  type Selection,
} from '../selection'

/** What a click on `hit` selects. */
export function selectionForClick(
  selection: Selection,
  hit: Hit | null,
  modifiers: Modifiers,
): Selection {
  if (!hit) return modifiers.shift || modifiers.mod ? selection : new Map()
  // Ctrl alone toggles; Ctrl+Alt is the mirror-handle *drag* modifier
  // (design spec §4.3) and must not also mean "toggle this out of the set".
  if (modifiers.mod && !modifiers.alt)
    return toggleSelection(selection, hit.target)
  if (modifiers.shift) return extendSelection(selection, hit.target)
  // Clicking inside the selection keeps it, so a drag can move the whole set.
  if (isSelected(selection, hit.target)) return selection
  return selectOne(hit.target)
}

/**
 * The cursor for what is under the pointer.
 *
 * `move` over a node, the insert cursor over a segment, the break-handle
 * cursor when Alt is held over a node, and the plain arrow over empty canvas.
 */
export function cursorForHover(hit: Hit | null, modifiers: Modifiers): string {
  if (!hit) return 'default'
  switch (hit.target.kind) {
    case 'node':
      // ⌥ over a node breaks its handle (§2.4, §4.3).
      return modifiers.alt ? toolCursor('breakHandle') : 'move'
    case 'handle':
    case 'anchor':
    case 'component':
      return 'move'
    case 'guide':
      return 'move'
    case 'segment':
      // ⌥ converts a line and ⇧ inserts a node, so both say "this click adds
      // something" with the insert-node cursor of §2.4.
      return modifiers.alt || modifiers.shift
        ? toolCursor('insertNode')
        : 'move'
  }
}
