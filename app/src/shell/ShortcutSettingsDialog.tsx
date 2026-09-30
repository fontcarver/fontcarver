import { useKeymapStore } from '../stores/keymapStore'
import { ShortcutSettingsForm } from './ShortcutSettingsForm'

/** Mount on demand so Cancel discards all edits and imports. */
export function ShortcutSettingsDialog() {
  const open = useKeymapStore((state) => state.open)
  return open ? <ShortcutSettingsForm /> : null
}
