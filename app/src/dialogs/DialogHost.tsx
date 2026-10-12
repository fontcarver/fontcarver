import { CompareFontsDialog } from '../views/crossdocument/CompareFontsDialog'
import { PasteSpecialDialog } from '../views/crossdocument/PasteSpecialDialog'
import { RelinkComponentsDialog } from '../views/crossdocument/RelinkComponentsDialog'
import { selectSummary, useSessionStore } from '../stores/sessionStore'
import { useUiStore } from '../stores/uiStore'

import { AddGlyphsDialog } from './AddGlyphsDialog'
import { BatchRenameDialog } from './BatchRenameDialog'
import { ExpandStrokeDialog } from './ExpandStrokeDialog'
import { ExportDialog } from './ExportDialog'
import { FindProblemsDialog } from './FindProblemsDialog'
import { NewFontDialog } from './NewFontDialog'
import { PreferencesDialog } from './PreferencesDialog'
import { RecoveryDialog } from './RecoveryDialog'
import { SimplifyDialog } from './SimplifyDialog'
import { StyleEffectsDialog } from '../views/style/StyleEffectsDialog'

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
    case 'compareFonts':
      return summary ? (
        <CompareFontsDialog
          key={dialog.doc}
          target={summary}
          onClose={() => useUiStore.getState().setDialog(null)}
        />
      ) : null
    case 'pasteSpecial':
      return summary ? (
        <PasteSpecialDialog
          key={dialog.doc}
          target={summary}
          glyphs={dialog.glyphs}
          onClose={() => useUiStore.getState().setDialog(null)}
        />
      ) : null
    case 'relinkComponents':
      return summary ? (
        <RelinkComponentsDialog
          key={dialog.doc}
          target={summary}
          glyphs={dialog.glyphs}
          onClose={() => useUiStore.getState().setDialog(null)}
        />
      ) : null
    case 'newFont':
      return <NewFontDialog />
    case 'batchRename':
      return summary ? (
        <BatchRenameDialog doc={dialog.doc} glyphs={dialog.glyphs} />
      ) : null
    case 'addGlyphs':
      return summary ? (
        <AddGlyphsDialog doc={dialog.doc} recipe={dialog.recipe} />
      ) : null
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
    case 'styleEffects':
      return summary ? (
        <StyleEffectsDialog
          key={dialog.doc}
          doc={dialog.doc}
          glyphs={dialog.glyphs}
          master={dialog.master}
          upem={summary.units_per_em}
          onClose={() => useUiStore.getState().setDialog(null)}
        />
      ) : null
  }
}
