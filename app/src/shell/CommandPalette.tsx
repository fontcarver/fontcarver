import { useUiStore } from '../stores/uiStore'

import { CommandPaletteDialog } from './CommandPaletteDialog'

/**
 * Mounts the command palette while it is open. Mounting on demand lets the
 * dialog capture the previously focused element during its first render, and
 * resets its query and highlight every time it opens.
 */
export function CommandPalette() {
  const open = useUiStore((state) => state.paletteOpen)
  return open ? <CommandPaletteDialog /> : null
}
