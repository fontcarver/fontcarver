import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { getTransport } from '../../api'
import { withSettledDocument } from '../../editor/documentPersistence'
import { saveArtifact } from '../../shell/saveArtifact'
import { reportError } from '../../shell/commands'
import { useDocumentStore } from '../../stores/documentStore'
import { useUiStore } from '../../stores/uiStore'
import {
  exportProofHtml,
  exportProofPdf,
  type ProofSheetFont,
} from './proofExport'
import { prepareProofSheet } from './prepareProofSheet'
import type { ProofEntry } from './proofLibrary'
import type { TextRequestContext } from './textLayout'
import styles from './ProofLibraryPanel.module.css'

/** Exports independent vector documents from complete, freshly shaped text. */
export function ProofExportControls({
  context,
  entries,
  title,
}: {
  context: TextRequestContext
  entries: readonly ProofEntry[]
  title: string
}) {
  const { t } = useTranslation()
  const [scope, setScope] = useState('current')
  const [rendering, setRendering] = useState<'font' | 'outlines'>('font')
  const [sizes, setSizes] = useState('24, 48, 72')
  const [busy, setBusy] = useState(false)
  const abort = useRef<AbortController | null>(null)
  useEffect(() => () => abort.current?.abort(), [])
  const values = sizes.split(',').map((value) => Number(value.trim()))
  const valid =
    values.length <= 8 &&
    values.every(
      (value) => Number.isFinite(value) && value >= 6 && value <= 256,
    )
  const save = async (format: 'html' | 'pdf') => {
    const controller = new AbortController()
    abort.current = controller
    setBusy(true)
    try {
      const samples: readonly ProofEntry[] =
        scope === 'library'
          ? entries
          : [
              {
                id: 'current',
                name: context.text,
                group: '',
                text: context.text,
                direction:
                  context.direction === 'rtl' || context.direction === 'ttb'
                    ? context.direction
                    : 'ltr',
                script: context.script ?? '',
                language: context.language ?? '',
              },
            ]
      const embedFont = format === 'html' && rendering === 'font'
      const { sheet, font } = await withSettledDocument(
        context.doc,
        async () => {
          const generation =
            useDocumentStore.getState().byDoc[context.doc]?.generation ?? 0
          const isCurrent = () =>
            generation ===
            (useDocumentStore.getState().byDoc[context.doc]?.generation ?? 0)
          const prepared = await prepareProofSheet(
            getTransport(),
            { ...context, generation },
            samples,
            values,
            { signal: controller.signal, isCurrent },
          )
          // The face is compiled inside the same barrier as the shaping, so the
          // embedded font and the text it sets come from one document revision.
          if (!embedFont) return { sheet: prepared, font: undefined }
          const compiled = await getTransport()
            .call('text.proof_font', {
              doc: context.doc,
              master: context.master,
              instance: null,
            })
            .catch(() => {
              throw new Error('proof.export.errors.font')
            })
          controller.signal.throwIfAborted()
          if (!isCurrent()) throw new Error('proof.export.errors.changed')
          const embedded: ProofSheetFont = {
            woff2Base64: compiled.woff2_base64,
            familyName: compiled.family_name,
            styleName: compiled.style_name,
            unitsPerEm: compiled.units_per_em,
          }
          return { sheet: prepared, font: embedded }
        },
      )
      controller.signal.throwIfAborted()
      const bytes =
        format === 'html'
          ? new TextEncoder().encode(exportProofHtml(title, sheet, font))
          : exportProofPdf(title, sheet)
      const saved = await saveArtifact(
        `${title.replace(/[^\p{L}\p{N}._ -]/gu, '_') || 'proof'}.${format}`,
        bytes,
        format === 'html' ? 'text/html;charset=utf-8' : 'application/pdf',
        t('proof.export.title'),
      )
      if (saved)
        useUiStore
          .getState()
          .pushToast({ kind: 'info', message: t('proof.export.saved') })
    } catch (error) {
      if (!controller.signal.aborted) {
        if (
          error instanceof Error &&
          error.message.startsWith('proof.export.errors.')
        )
          useUiStore
            .getState()
            .pushToast({ kind: 'error', message: t(error.message) })
        else reportError(error)
      }
    } finally {
      if (abort.current === controller) {
        abort.current = null
        setBusy(false)
      }
    }
  }
  return (
    <div
      className={styles.row}
      role="group"
      aria-label={t('proof.export.title')}
    >
      <label>
        {t('proof.export.scope')}
        <select
          value={scope}
          onChange={(event) => setScope(event.currentTarget.value)}
          disabled={busy}
        >
          <option value="current">{t('proof.export.current')}</option>
          <option value="library">{t('proof.export.library')}</option>
        </select>
      </label>
      <label>
        {t('proof.export.sizes')}
        <input
          value={sizes}
          onChange={(event) => setSizes(event.currentTarget.value)}
          aria-invalid={!valid}
          disabled={busy}
        />
      </label>
      <label>
        {t('proof.export.rendering')}
        <select
          value={rendering}
          title={t('proof.export.renderingHint')}
          onChange={(event) =>
            setRendering(
              event.currentTarget.value === 'outlines' ? 'outlines' : 'font',
            )
          }
          disabled={busy}
        >
          <option value="font">{t('proof.export.renderingFont')}</option>
          <option value="outlines">
            {t('proof.export.renderingOutlines')}
          </option>
        </select>
      </label>
      {(['html', 'pdf'] as const).map((format) => (
        <button
          key={format}
          type="button"
          disabled={
            busy ||
            !valid ||
            (scope === 'library' ? !entries.length : !context.text)
          }
          onClick={() => void save(format)}
        >
          {t(`proof.export.${format}`)}
        </button>
      ))}
      {busy && (
        <>
          <span role="status">{t('proof.export.busy')}</span>
          <button type="button" onClick={() => abort.current?.abort()}>
            {t('proof.export.cancel')}
          </button>
        </>
      )}
    </div>
  )
}
