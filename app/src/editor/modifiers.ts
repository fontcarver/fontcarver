import { isMacPlatform } from '../shell/shortcuts'

import type { Modifiers } from './gestures/drag'

/** Browser modifier state before assigning platform-specific meanings. */
interface ModifierEvent {
  shiftKey: boolean
  altKey: boolean
  ctrlKey: boolean
  metaKey: boolean
}

/** Keeps physical Control separate from the platform command modifier. */
export function modifiersOf(
  event: ModifierEvent,
  mac: boolean = isMacPlatform(),
): Modifiers {
  return {
    shift: event.shiftKey,
    alt: event.altKey,
    mod: mac ? event.metaKey : event.ctrlKey,
    ctrl: event.ctrlKey,
  }
}
