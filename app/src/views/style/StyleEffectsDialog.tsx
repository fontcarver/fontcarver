import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  call,
  type DocId,
  type GlyphId,
  type StyleBatchParams,
  type StyleBatchResult,
} from '../../api'
import { Button } from '../../components/Button'
import { Dialog } from '../../components/Dialog'
import { Select } from '../../components/Select'
import { errorNotice } from '../../i18n/errors'
import {
  STYLE_PRESETS,
  presetFields,
  styleOptions,
  type StyleFields,
  type StylePreset,
} from './stylePresets'
import { StylePreview } from './StylePreview'
import styles from './styleEffects.module.css'

const FIELDS: {
  key: keyof StyleFields
  min: number
  max: number
  step: number
}[] = [
  { key: 'scaleX', min: 5, max: 400, step: 1 },
  { key: 'scaleY', min: 5, max: 400, step: 1 },
  { key: 'weightX', min: -500, max: 500, step: 0.1 },
  { key: 'weightY', min: -500, max: 500, step: 0.1 },
  { key: 'oblique', min: -60, max: 60, step: 0.1 },
  { key: 'moveX', min: -32768, max: 32768, step: 1 },
  { key: 'moveY', min: -32768, max: 32768, step: 1 },
]

/** Baked style operations on a captured selection, reviewed before atomic apply. */
export function StyleEffectsDialog({
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
  const [preset, setPreset] = useState<StylePreset | 'custom'>('weight')
  const [fields, setFields] = useState(() => presetFields('weight', upem))
  const [allMasters, setAllMasters] = useState(false)
  const [scaleAdvance, setScaleAdvance] = useState(false)
  const [center, setCenter] = useState(false)
  const [backup, setBackup] = useState(true)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [preview, setPreview] = useState<{
    params: StyleBatchParams
    result: StyleBatchResult
  } | null>(null)
  const invalidate = () => {
    setPreview(null)
    setError(null)
  }
  const previewChanges = async () => {
    setBusy(true)
    setPreview(null)
    setError(null)
    try {
      const options = styleOptions(fields)
      if (!options) throw new Error(t('styleEffects.invalidNumber'))
      const params: StyleBatchParams = {
        doc,
        glyphs,
        master: allMasters ? null : master,
        options,
        scale_advance: scaleAdvance,
        center,
        backup,
        dry_run: true,
        expected_generation: null,
        label: t('styleEffects.title'),
      }
      const result = await call('style.batch', params)
      setPreview({ params, result })
    } catch (error) {
      const notice = errorNotice(error, t)
      setError([notice.message, notice.details].filter(Boolean).join(' · '))
    } finally {
      setBusy(false)
    }
  }
  const apply = async () => {
    if (!preview) return
    setBusy(true)
    setError(null)
    try {
      await call('style.batch', {
        ...preview.params,
        dry_run: false,
        expected_generation: preview.result.generation,
      })
      onClose()
    } catch (error) {
      setPreview(null)
      const notice = errorNotice(error, t)
      setError([notice.message, notice.details].filter(Boolean).join(' · '))
    } finally {
      setBusy(false)
    }
  }
  return (
    <Dialog
      title={t('styleEffects.title')}
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
      <div className={styles.editor} aria-busy={busy}>
        <p>{t('styleEffects.selection', { count: glyphs.length })}</p>
        <p>{t('styleEffects.scope')}</p>
        <fieldset className={styles.form} disabled={busy} onChange={invalidate}>
          <label className={styles.preset}>
            {t('styleEffects.preset')}
            <Select
              label={t('styleEffects.preset')}
              value={preset}
              options={[
                ...STYLE_PRESETS.map((value) => ({
                  value,
                  label: t(`styleEffects.presets.${value}`),
                })),
                { value: 'custom', label: t('styleEffects.custom') },
              ]}
              onChange={(value) => {
                setPreset(value)
                if (value !== 'custom') {
                  setFields(presetFields(value, upem))
                  setScaleAdvance(
                    [
                      'condense',
                      'extend',
                      'smallCaps',
                      'subscript',
                      'superscript',
                    ].includes(value),
                  )
                }
              }}
            />
          </label>
          <div className={styles.numbers}>
            {FIELDS.map(({ key, ...range }) => (
              <label key={key}>
                {t(`styleEffects.fields.${key}`)}
                <input
                  type="number"
                  {...range}
                  value={fields[key]}
                  onChange={(event) => {
                    setFields({ ...fields, [key]: event.target.value })
                    setPreset('custom')
                  }}
                />
              </label>
            ))}
          </div>
          <p>{t('styleEffects.weightHelp')}</p>
          <div className={styles.toggles}>
            <label>
              <input
                type="checkbox"
                checked={allMasters}
                onChange={(event) => setAllMasters(event.target.checked)}
              />
              {t('cjk.allMasters')}
            </label>
            <label>
              <input
                type="checkbox"
                checked={scaleAdvance}
                onChange={(event) => setScaleAdvance(event.target.checked)}
              />
              {t('styleEffects.scaleAdvance')}
            </label>
            <label>
              <input
                type="checkbox"
                checked={center}
                onChange={(event) => setCenter(event.target.checked)}
              />
              {t('styleEffects.center')}
            </label>
            <label>
              <input
                type="checkbox"
                checked={backup}
                onChange={(event) => setBackup(event.target.checked)}
              />
              {t('styleEffects.backup')}
            </label>
          </div>
        </fieldset>
        <p>{t('styleEffects.variableHelp')}</p>
        {error && (
          <p role="alert" className={styles.error}>
            {error}
          </p>
        )}
        {preview && (
          <section aria-label={t('styleEffects.review')}>
            <p role="status">
              {t('cjk.batchCount', {
                count: preview.result.layer_count,
                glyphs: preview.result.glyph_count,
              })}{' '}
              {t('styleEffects.decomposed', {
                count: preview.result.decomposed_layers,
              })}
            </p>
            <p>{t('styleEffects.examples')}</p>
            {preview.result.approximated_layers > 0 && (
              <p className={styles.warning} role="note">
                {t('styleEffects.approximated', {
                  count: preview.result.approximated_layers,
                })}{' '}
                {preview.result.approximated_sources.join(' · ')}
              </p>
            )}
            {preview.result.examples.map((example, index) => (
              <StylePreview key={index} example={example} upem={upem} />
            ))}
          </section>
        )}
      </div>
    </Dialog>
  )
}
