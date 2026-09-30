import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  call,
  type DocId,
  type DocSummary,
  type GlyphId,
  type PasteSpecialParams,
  type PasteSpecialResult,
} from '../../api'
import { Button } from '../../components/Button'
import { Dialog } from '../../components/Dialog'
import { withSettledDocument } from '../../editor/documentPersistence'
import { errorNotice } from '../../i18n/errors'
import { useDocumentStore } from '../../stores/documentStore'
import { useSessionStore } from '../../stores/sessionStore'
import {
  documentGlyphs,
  mapPasteTargets,
  parseGlyphNames,
} from './crossDocumentModel'
import { DocumentMasterFields } from './DocumentMasterFields'
import styles from './crossDocument.module.css'

/** Reviewed cross-document paste, with both document generations captured. */
export function PasteSpecialDialog({
  target,
  glyphs,
  onClose,
}: {
  target: DocSummary
  glyphs: GlyphId[]
  onClose: () => void
}) {
  const { t } = useTranslation()
  const docs = useSessionStore((state) => state.docs)
  const initialSource = docs.find((doc) => doc.doc !== target.doc)
  const [sourceDoc, setSourceDoc] = useState<DocId | null>(
    initialSource?.doc ?? null,
  )
  const [sourceMaster, setSourceMaster] = useState(
    initialSource?.masters[0]?.id ?? '',
  )
  const [targetMaster, setTargetMaster] = useState(target.masters[0]?.id ?? '')
  const [names, setNames] = useState('')
  const [destination, setDestination] =
    useState<PasteSpecialParams['destination']>('same_name')
  const [content, setContent] =
    useState<PasteSpecialParams['content']>('all_data')
  const [missingBases, setMissingBases] =
    useState<PasteSpecialParams['missing_bases']>('copy')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [preview, setPreview] = useState<{
    params: PasteSpecialParams
    result: PasteSpecialResult
  } | null>(null)
  const invalidate = () => {
    setPreview(null)
    setError(null)
  }
  const failure = (reason: unknown) => {
    const notice = errorNotice(reason, t)
    setError([notice.message, notice.details].filter(Boolean).join(' · '))
  }
  const review = async () => {
    if (sourceDoc === null) return
    setBusy(true)
    invalidate()
    try {
      let requested: string[]
      try {
        requested = parseGlyphNames(names)
      } catch {
        throw new Error(t('crossDocument.duplicateNames'))
      }
      const inventory = await documentGlyphs(sourceDoc)
      const byName = new Map(inventory.map((glyph) => [glyph.name, glyph.gid]))
      const selection = [
        ...(useDocumentStore.getState().byDoc[sourceDoc]?.selection ?? []),
      ]
      const sources =
        requested.length > 0
          ? requested.map((name) => {
              const gid = byName.get(name)
              if (gid === undefined)
                throw new Error(t('crossDocument.unknownGlyph', { name }))
              return gid
            })
          : selection
      if (sources.length === 0)
        throw new Error(t('crossDocument.noSourceGlyphs'))
      let mappings: PasteSpecialParams['mappings']
      try {
        mappings = mapPasteTargets(sources, glyphs, destination)
      } catch {
        throw new Error(
          t('crossDocument.selectionCount', {
            source: sources.length,
            target: glyphs.length,
          }),
        )
      }
      const params: PasteSpecialParams = {
        doc: target.doc,
        source_doc: sourceDoc,
        mappings,
        destination,
        content,
        source_master: sourceMaster || null,
        target_master: targetMaster || null,
        missing_bases: missingBases,
        dry_run: true,
        expected_generation: null,
        expected_source_generation: null,
      }
      const result = await withSettledDocument(sourceDoc, () =>
        withSettledDocument(target.doc, () =>
          call('doc.paste_special', params),
        ),
      )
      setPreview({ params, result })
    } catch (reason) {
      failure(reason)
    } finally {
      setBusy(false)
    }
  }
  const apply = async () => {
    if (!preview) return
    setBusy(true)
    setError(null)
    try {
      await withSettledDocument(preview.params.source_doc, () =>
        withSettledDocument(target.doc, () =>
          call('doc.paste_special', {
            ...preview.params,
            dry_run: false,
            expected_generation: preview.result.generation,
            expected_source_generation: preview.result.source_generation,
          }),
        ),
      )
      await useSessionStore.getState().refreshSummary(target.doc)
      onClose()
    } catch (reason) {
      setPreview(null)
      failure(reason)
    } finally {
      setBusy(false)
    }
  }
  return (
    <Dialog
      title={t('crossDocument.pasteTitle')}
      size="lg"
      scrollBody
      onCancel={() => {
        if (!busy) onClose()
      }}
      footer={
        <>
          <Button disabled={busy} onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button
            disabled={busy || sourceDoc === null}
            onClick={() => {
              void review()
            }}
          >
            {t('crossDocument.preview')}
          </Button>
          <Button
            variant="primary"
            disabled={busy || !preview}
            onClick={() => {
              void apply()
            }}
          >
            {t('crossDocument.apply')}
          </Button>
        </>
      }
    >
      <div className={styles.form} aria-busy={busy}>
        <DocumentMasterFields
          docs={docs}
          sourceDoc={sourceDoc}
          sourceMaster={sourceMaster}
          target={target}
          targetMaster={targetMaster}
          disabled={busy}
          onSource={(doc) => {
            setSourceDoc(doc)
            setSourceMaster(
              docs.find((item) => item.doc === doc)?.masters[0]?.id ?? '',
            )
            invalidate()
          }}
          onSourceMaster={(master) => {
            setSourceMaster(master)
            invalidate()
          }}
          onTargetMaster={(master) => {
            setTargetMaster(master)
            invalidate()
          }}
        />
        {docs.length < 2 && <p>{t('crossDocument.openSourceHelp')}</p>}
        <fieldset className={styles.form} disabled={busy} onChange={invalidate}>
          <label className={styles.field}>
            {t('crossDocument.glyphNames')}
            <textarea
              value={names}
              onChange={(event) => setNames(event.target.value)}
            />
          </label>
          <p className={styles.help}>
            {t('crossDocument.sourceSelectionHelp')}
          </p>
          <div className={styles.row}>
            <label className={styles.field}>
              {t('crossDocument.destination')}
              <select
                value={destination}
                onChange={(event) =>
                  setDestination(event.target.value as typeof destination)
                }
              >
                {(['same_name', 'selected', 'components'] as const).map(
                  (value) => (
                    <option key={value} value={value}>
                      {t(`crossDocument.destinations.${value}`)}
                    </option>
                  ),
                )}
              </select>
            </label>
            <label className={styles.field}>
              {t('crossDocument.content')}
              <select
                value={content}
                onChange={(event) =>
                  setContent(event.target.value as typeof content)
                }
              >
                {(
                  [
                    'all_data',
                    'layer',
                    'kerning_groups',
                    'metric_keys',
                  ] as const
                ).map((value) => (
                  <option key={value} value={value}>
                    {t(`crossDocument.contents.${value}`)}
                  </option>
                ))}
              </select>
            </label>
          </div>
          <p className={styles.help}>
            {t('crossDocument.targetSelection', { count: glyphs.length })}
          </p>
          <label className={styles.field}>
            {t('crossDocument.missingBases')}
            <select
              value={missingBases}
              onChange={(event) =>
                setMissingBases(event.target.value as typeof missingBases)
              }
            >
              {(['copy', 'placeholder', 'reject', 'decompose'] as const).map(
                (value) => (
                  <option key={value} value={value}>
                    {t(`crossDocument.policies.${value}`)}
                  </option>
                ),
              )}
            </select>
          </label>
        </fieldset>
        {error && (
          <p role="alert" className={styles.error}>
            {error}
          </p>
        )}
        {preview && (
          <section aria-label={t('crossDocument.review')}>
            <p role="status">
              {t('crossDocument.pasteCount', {
                count: preview.result.pasted.length,
                bases: preview.result.added_bases.length,
              })}
            </p>
            <p>{preview.result.pasted.join(', ')}</p>
            {preview.result.added_bases.length > 0 && (
              <p>
                {t('crossDocument.addedBases', {
                  names: preview.result.added_bases.join(', '),
                })}
              </p>
            )}
            {preview.result.warnings.map((warning, index) => (
              <p key={index} role="note">
                {warning}
              </p>
            ))}
          </section>
        )}
      </div>
    </Dialog>
  )
}
