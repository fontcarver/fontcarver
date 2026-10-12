import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  call,
  type CjkBatchParams,
  type CjkBatchResult,
  type DocId,
  type GlyphId,
} from '../../api'
import { Button } from '../../components/Button'
import { Dialog } from '../../components/Dialog'
import { reportError } from '../../stores/uiStore'
import styles from '../fontinfo/designSpace.module.css'

/** Preview captured glyph selection metrics before one revision-checked edit. */
export function CjkBatchDialog({
  doc,
  glyphs,
  master,
  upem,
  onClose,
}: {
  doc: DocId
  glyphs: GlyphId[]
  master: string
  upem: number
  onClose: () => void
}) {
  const { t } = useTranslation()
  const [allMasters, setAllMasters] = useState(false)
  const [widthEnabled, setWidthEnabled] = useState(true)
  const [width, setWidth] = useState(String(upem))
  const [center, setCenter] = useState(false)
  const [vertical, setVertical] = useState(false)
  const [advance, setAdvance] = useState(String(upem))
  const [origin, setOrigin] = useState('')
  const [busy, setBusy] = useState(false)
  const [preview, setPreview] = useState<{
    params: CjkBatchParams
    result: CjkBatchResult
  } | null>(null)
  const invalidate = () => setPreview(null)
  const previewChanges = async () => {
    setBusy(true)
    try {
      const number = (text: string, optional: boolean) => {
        if (text.trim() === '' && optional) return null
        const value = Number(text)
        if (text.trim() === '' || !Number.isFinite(value))
          throw new Error(t('cjk.batchInvalid'))
        return value
      }
      const params: CjkBatchParams = {
        doc,
        glyphs,
        master: allMasters ? null : master,
        width: widthEnabled ? number(width, false) : null,
        center,
        vertical,
        advance: vertical ? number(advance, true) : null,
        origin: vertical ? number(origin, true) : null,
        dry_run: true,
        expected_generation: null,
        label: t('cjk.batchTitle'),
      }
      const result = await call('cjk.batch', params)
      setPreview({ params, result })
    } catch (error) {
      reportError(error)
    } finally {
      setBusy(false)
    }
  }
  const apply = async () => {
    if (!preview) return
    setBusy(true)
    try {
      await call('cjk.batch', {
        ...preview.params,
        dry_run: false,
        expected_generation: preview.result.generation,
      })
      onClose()
    } catch (error) {
      setPreview(null)
      reportError(error)
    } finally {
      setBusy(false)
    }
  }
  return (
    <Dialog
      title={t('cjk.batchTitle')}
      onCancel={() => {
        if (!busy) onClose()
      }}
      size="lg"
      scrollBody
      footer={
        <>
          <Button disabled={busy} onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button
            disabled={busy}
            onClick={() => {
              void previewChanges()
            }}
          >
            {t('cjk.batchPreview')}
          </Button>
          <Button
            variant="primary"
            disabled={busy || !preview}
            onClick={() => {
              void apply()
            }}
          >
            {t('variation.apply')}
          </Button>
        </>
      }
    >
      <div className={styles.editor}>
        <p>{t('cjk.batchSelection', { count: glyphs.length })}</p>
        <fieldset className={styles.form} disabled={busy} onChange={invalidate}>
          <div className={styles.row}>
            <label>
              <input
                type="checkbox"
                checked={allMasters}
                onChange={(e) => setAllMasters(e.target.checked)}
              />
              {t('cjk.allMasters')}
            </label>
          </div>
          <div className={styles.row}>
            <label>
              <input
                type="checkbox"
                checked={widthEnabled}
                onChange={(e) => setWidthEnabled(e.target.checked)}
              />
              {t('cjk.setWidth')}
            </label>
            <input
              type="number"
              min={0}
              max={65535}
              aria-label={t('cjk.width')}
              disabled={!widthEnabled}
              value={width}
              onChange={(e) => setWidth(e.target.value)}
            />
            <label>
              <input
                type="checkbox"
                checked={center}
                onChange={(e) => setCenter(e.target.checked)}
              />
              {t('cjk.center')}
            </label>
          </div>
          <div className={styles.row}>
            <label>
              <input
                type="checkbox"
                checked={vertical}
                onChange={(e) => setVertical(e.target.checked)}
              />
              {t('cjk.setVertical')}
            </label>
          </div>
          <div className={styles.row}>
            <label>
              {t('cjk.advance')}
              <input
                type="number"
                min={0}
                max={65535}
                disabled={!vertical}
                value={advance}
                onChange={(e) => setAdvance(e.target.value)}
              />
            </label>
            <label>
              {t('cjk.origin')}
              <input
                type="number"
                min={-32768}
                max={32767}
                disabled={!vertical}
                value={origin}
                onChange={(e) => setOrigin(e.target.value)}
              />
            </label>
          </div>
          <p>{t('cjk.verticalDefault')}</p>
        </fieldset>
        {preview && (
          <div role="status">
            <p>
              {t('cjk.batchCount', {
                count: preview.result.layer_count,
                glyphs: preview.result.glyph_count,
              })}
            </p>
            <ul>
              {preview.result.examples.map((example, index) => (
                <li key={index}>
                  {example.glyph} · {example.master}: {example.before} →{' '}
                  {example.after}
                </li>
              ))}
            </ul>
          </div>
        )}
      </div>
    </Dialog>
  )
}
