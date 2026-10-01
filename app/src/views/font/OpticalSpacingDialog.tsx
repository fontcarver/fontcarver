import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  call,
  type DocId,
  type GlyphId,
  type OpticalBatchParams,
  type OpticalBatchResult,
  type OpticalOperation,
} from '../../api'
import { Button } from '../../components/Button'
import { Dialog } from '../../components/Dialog'
import { Select } from '../../components/Select'
import { withSettledDocument } from '../../editor/documentPersistence'
import { errorNotice } from '../../i18n/errors'
import { OpticalMetricsPreview } from './OpticalMetricsPreview'
import { OpticalProposalTable, proposalKey } from './OpticalProposalTable'
import styles from './opticalSpacing.module.css'

/** Preview a captured glyph selection before one generation-checked optical edit. */
export function OpticalSpacingDialog({
  doc,
  glyphs,
  master,
  onClose,
}: {
  doc: DocId
  glyphs: GlyphId[]
  master: string
  onClose: () => void
}) {
  const { t } = useTranslation()
  const densityHelp = useId()
  const [captured] = useState(() => ({
    doc,
    glyphs: [...new Set(glyphs)],
    master,
  }))
  const [operation, setOperation] = useState<OpticalOperation>('spacing')
  const [density, setDensity] = useState('1')
  const [allMasters, setAllMasters] = useState(false)
  const [replaceExisting, setReplaceExisting] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [preview, setPreview] = useState<{
    params: OpticalBatchParams
    result: OpticalBatchResult
  } | null>(null)
  // Rejected rows of the reviewed proposal, keyed as the kernel names them.
  const [rejected, setRejected] = useState<ReadonlySet<string>>(new Set())
  const accepted = (preview?.result.proposals ?? []).filter(
    (entry) => !rejected.has(proposalKey(entry)),
  )
  const validSelection =
    captured.glyphs.length > 0 &&
    captured.glyphs.length <= 128 &&
    captured.master !== ''
  const invalidate = () => {
    setPreview(null)
    setError(null)
    setRejected(new Set())
  }
  const showError = (reason: unknown) => {
    const notice = errorNotice(reason, t)
    setError([notice.message, notice.details].filter(Boolean).join(' · '))
  }
  const previewChanges = async () => {
    setBusy(true)
    invalidate()
    try {
      if (!validSelection) throw new Error(t('opticalSpacing.invalidSelection'))
      const value = Number(density)
      if (
        density.trim() === '' ||
        !Number.isFinite(value) ||
        value < 0.25 ||
        value > 4
      )
        throw new Error(t('opticalSpacing.invalidDensity'))
      const params: OpticalBatchParams = {
        doc: captured.doc,
        glyphs: captured.glyphs,
        master: allMasters ? null : captured.master,
        operation,
        density: value,
        replace_existing: operation === 'kerning' && replaceExisting,
        accepted: null,
        dry_run: true,
        expected_generation: null,
        label: t('opticalSpacing.title'),
      }
      const result = await withSettledDocument(captured.doc, () =>
        call('optical.batch', params),
      )
      setPreview({ params, result })
    } catch (reason) {
      showError(reason)
    } finally {
      setBusy(false)
    }
  }
  const apply = async () => {
    if (!preview || accepted.length === 0) return
    setBusy(true)
    setError(null)
    try {
      await withSettledDocument(captured.doc, () =>
        call('optical.batch', {
          ...preview.params,
          // The reviewed rows are named explicitly, so a proposal that no longer
          // holds one of them is refused rather than partly applied.
          accepted: accepted.map((entry) => ({
            glyph: entry.glyph,
            right: entry.right,
            master: entry.master,
          })),
          dry_run: false,
          expected_generation: preview.result.generation,
        }),
      )
      onClose()
    } catch (reason) {
      setPreview(null)
      showError(reason)
    } finally {
      setBusy(false)
    }
  }
  const skipped = new Map<string, number>()
  for (const item of preview?.result.skipped ?? []) {
    skipped.set(item.reason, (skipped.get(item.reason) ?? 0) + 1)
  }

  return (
    <Dialog
      title={t('opticalSpacing.title')}
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
            disabled={busy || !validSelection}
            onClick={() => void previewChanges()}
          >
            {t('opticalSpacing.preview')}
          </Button>
          <Button
            variant="primary"
            disabled={busy || !preview || accepted.length === 0}
            onClick={() => void apply()}
          >
            {t('opticalSpacing.apply')}
          </Button>
        </>
      }
    >
      <div className={styles.editor} aria-busy={busy}>
        <p>
          {t('opticalSpacing.selection', { count: captured.glyphs.length })}
        </p>
        <p>{t('opticalSpacing.scope')}</p>
        {!validSelection && (
          <p role="alert" className={styles.error}>
            {t('opticalSpacing.invalidSelection')}
          </p>
        )}
        <fieldset className={styles.form} disabled={busy} onChange={invalidate}>
          <label className={styles.row}>
            {t('opticalSpacing.operation')}
            <Select
              label={t('opticalSpacing.operation')}
              value={operation}
              options={(['spacing', 'kerning'] as const).map((value) => ({
                value,
                label: t(`opticalSpacing.${value}`),
              }))}
              onChange={setOperation}
            />
          </label>
          <div className={styles.density}>
            <label className={styles.row}>
              {t('opticalSpacing.density')}
              <input
                type="number"
                min={0.25}
                max={4}
                step={0.05}
                value={density}
                aria-describedby={densityHelp}
                onChange={(event) => setDensity(event.target.value)}
              />
            </label>
            <p id={densityHelp}>{t('opticalSpacing.densityHelp')}</p>
          </div>
          <label className={styles.toggle}>
            <input
              type="checkbox"
              checked={allMasters}
              onChange={(event) => setAllMasters(event.target.checked)}
            />
            {t('opticalSpacing.allMasters')}
          </label>
          {operation === 'kerning' && (
            <>
              <label className={styles.toggle}>
                <input
                  type="checkbox"
                  checked={replaceExisting}
                  onChange={(event) => setReplaceExisting(event.target.checked)}
                />
                {t('opticalSpacing.replaceExisting')}
              </label>
              <p>{t('opticalSpacing.kerningHelp')}</p>
            </>
          )}
        </fieldset>
        <p>{t('opticalSpacing.samplingHelp')}</p>
        {error && (
          <p role="alert" className={styles.error}>
            {error}
          </p>
        )}
        {preview && (
          <section
            className={styles.review}
            aria-label={t('opticalSpacing.review')}
          >
            <p role="status">
              {t(`opticalSpacing.${preview.params.operation}Count`, {
                count: preview.result.changed_count,
                glyphs: preview.result.glyph_count,
              })}
            </p>
            {preview.result.changed_count === 0 ? (
              <p>{t('opticalSpacing.noChanges')}</p>
            ) : (
              <>
                <p>
                  {t('opticalSpacing.examples', {
                    shown: Math.min(preview.result.examples.length, 16),
                    total: preview.result.changed_count,
                  })}
                </p>
                <OpticalMetricsPreview
                  examples={preview.result.examples}
                  operation={preview.params.operation}
                />
                <OpticalProposalTable
                  proposals={preview.result.proposals}
                  rejected={rejected}
                  onRejected={setRejected}
                />
                {accepted.length === 0 && (
                  <p role="alert" className={styles.error}>
                    {t('opticalSpacing.nothingAccepted')}
                  </p>
                )}
              </>
            )}
            {skipped.size > 0 && (
              <>
                <p>
                  {t(
                    preview.params.operation === 'spacing'
                      ? 'opticalSpacing.skippedLayers'
                      : 'opticalSpacing.skippedPairs',
                    { count: preview.result.skipped.length },
                  )}
                </p>
                <ul className={styles.skipped}>
                  {[...skipped].map(([reason, count]) => (
                    <li key={reason}>
                      {t('opticalSpacing.skipCount', {
                        reason: t(`opticalSpacing.skipReason.${reason}`),
                        count,
                      })}
                    </li>
                  ))}
                </ul>
              </>
            )}
          </section>
        )}
      </div>
    </Dialog>
  )
}
