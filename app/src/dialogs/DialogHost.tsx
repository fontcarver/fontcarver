import { selectSummary, useSessionStore } from '../stores/sessionStore'
import { useUiStore } from '../stores/uiStore'

import { AddGlyphsDialog } from './AddGlyphsDialog'
import { ExportDialog } from './ExportDialog'
import { NewFontDialog } from './NewFontDialog'
import { PreferencesDialog } from './PreferencesDialog'
import { RecoveryDialog } from './RecoveryDialog'

/**
 * Mounts whichever modal dialog the UI store has open. Dialogs are mounted, not
 * hidden, so each one starts from a clean form and its focus trap exists only
 * while it is on screen.
 */
export function DialogHost() {
  const dialog = useUiStore((state) => state.dialog)
  const doc = dialog && 'doc' in dialog ? dialog.doc : null
  const summary = useSessionStore(selectSummary(doc))

  if (!dialog) return null
  switch (dialog.kind) {
    case 'newFont':
      return <NewFontDialog />
    case 'addGlyphs':
      return summary ? <AddGlyphsDialog doc={dialog.doc} /> : null
    case 'export':
      return summary ? <ExportDialog summary={summary} /> : null
    case 'preferences':
      return <PreferencesDialog />
    case 'recovery':
      return <RecoveryDialog entries={dialog.entries} />
  }
}
