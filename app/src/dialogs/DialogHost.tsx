import { selectSummary, useSessionStore } from '../stores/sessionStore'
import { useUiStore } from '../stores/uiStore'

import { AddGlyphsDialog } from './AddGlyphsDialog'
import { ExpandStrokeDialog } from './ExpandStrokeDialog'
import { ExportDialog } from './ExportDialog'
import { FindProblemsDialog } from './FindProblemsDialog'
import { NewFontDialog } from './NewFontDialog'
import { PreferencesDialog } from './PreferencesDialog'
import { RecoveryDialog } from './RecoveryDialog'
import { SimplifyDialog } from './SimplifyDialog'

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
      return summary ? (
        <ExportDialog key={summary.doc} summary={summary} />
      ) : null
    case 'findProblems':
      return summary ? (
        <FindProblemsDialog key={summary.doc} summary={summary} />
      ) : null
    case 'preferences':
      return <PreferencesDialog />
    case 'recovery':
      return <RecoveryDialog entries={dialog.entries} />
    case 'simplify':
      return <SimplifyDialog />
    case 'expandStroke':
      return <ExpandStrokeDialog />
  }
}
