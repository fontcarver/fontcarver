import { registerCommands } from '../../shell/commands'
import { useDocumentStore } from '../../stores/documentStore'
import { activeDocId, useUiStore } from '../../stores/uiStore'

/** Expose document comparison, transfer and component repair in shell menus. */
export function registerCrossDocumentCommands(): void {
  registerCommands([
    {
      id: 'font.compare',
      title: 'crossDocument.compareTitle',
      menu: 'font',
      group: 'documents',
      when: () => activeDocId() !== null,
      disabledReason: 'cmd.reason.noDocument',
      run: () => {
        const doc = activeDocId()
        if (doc !== null)
          useUiStore.getState().setDialog({ kind: 'compareFonts', doc })
      },
    },
    ...(['pasteSpecial', 'relinkComponents'] as const).map((kind) => ({
      id: `edit.${kind}`,
      title:
        kind === 'pasteSpecial'
          ? 'crossDocument.pasteTitle'
          : 'crossDocument.relinkTitle',
      menu: 'edit' as const,
      group: 'documents',
      when: () => activeDocId() !== null,
      disabledReason: 'cmd.reason.noDocument',
      run: () => {
        const doc = activeDocId()
        if (doc !== null)
          useUiStore.getState().setDialog({
            kind,
            doc,
            glyphs: [
              ...(useDocumentStore.getState().byDoc[doc]?.selection ?? []),
            ],
          })
      },
    })),
  ])
}
