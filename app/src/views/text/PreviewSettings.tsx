import { useEffect, useId, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { PreviewStripProps } from './presentationTypes'
import {
  formatWaterfallSizes,
  parseWaterfallSizes,
  MAX_WATERFALL_STEPS,
  type WaterfallReason,
} from './waterfallSizes'
import styles from './PreviewStrip.module.css'

/** Preview settings remain mounted while their disclosure is closed. */
export function PreviewSettings({
  settings,
  onSettingsChange,
  instances,
}: Pick<PreviewStripProps, 'settings' | 'onSettingsChange' | 'instances'>) {
  const { t } = useTranslation()
  const canonical = useMemo(
    () => formatWaterfallSizes(settings.sizes),
    [settings.sizes],
  )
  const [sizeDraft, setSizeDraft] = useState(canonical)
  // What the last attempt to apply the field produced: a refusal reason, a note
  // that a long range was cut, or nothing at all.
  const [sizeNotice, setSizeNotice] = useState<
    WaterfallReason | 'truncated' | null
  >(null)
  const editingSizes = useRef(false)
  // The list this strip itself just applied. The settings coming back are not a
  // new list from elsewhere, so they must not wipe the note that explains them.
  const applied = useRef('')
  const sizeErrorId = useId()
  // A list arriving from the workspace or the docked strip replaces the draft,
  // but never while it is being typed into.
  useEffect(() => {
    if (editingSizes.current || canonical === applied.current) return
    setSizeDraft(canonical)
    setSizeNotice(null)
  }, [canonical])
  function applySizes() {
    const parsed = parseWaterfallSizes(sizeDraft)
    if (!parsed.sizes) {
      setSizeNotice(parsed.reason)
      return
    }
    editingSizes.current = false
    const canonicalised = formatWaterfallSizes(parsed.sizes)
    applied.current = canonicalised
    setSizeNotice(parsed.truncated ? 'truncated' : null)
    setSizeDraft(canonicalised)
    onSettingsChange({ sizes: parsed.sizes })
  }
  const rejected = sizeNotice !== null && sizeNotice !== 'truncated'

  return (
    <>
      <form
        className={styles.controls}
        onSubmit={(event) => {
          event.preventDefault()
          applySizes()
        }}
      >
        {instances && instances.length > 1 && (
          <label>
            {t('proofPreview.instance')}
            <select
              value={settings.instance}
              onChange={(event) =>
                onSettingsChange({ instance: event.target.value })
              }
            >
              <optgroup label={t('proofPreview.masters')}>
                {instances
                  .filter((option) => option.kind === 'master')
                  .map((option) => (
                    <option key={option.id} value={option.id}>
                      {option.name}
                    </option>
                  ))}
              </optgroup>
              {instances.some((option) => option.kind === 'instance') && (
                <optgroup label={t('proofPreview.instances')}>
                  {instances
                    .filter((option) => option.kind === 'instance')
                    .map((option) => (
                      <option key={option.id} value={option.id}>
                        {option.name}
                      </option>
                    ))}
                </optgroup>
              )}
            </select>
          </label>
        )}
        <label>
          {t('proofPreview.mode')}
          <select
            value={settings.mode}
            onChange={(event) =>
              onSettingsChange({
                mode: event.target.value === 'single' ? 'single' : 'run',
              })
            }
          >
            <option value="run">{t('proofPreview.run')}</option>
            <option value="single">{t('proofPreview.single')}</option>
          </select>
        </label>
        <label>
          {t('proofPreview.blur')}
          <input
            className={styles.blurInput}
            type="number"
            min={0}
            max={4}
            step={0.25}
            value={settings.blur}
            onChange={(event) => {
              const value = Number(event.target.value)
              onSettingsChange({
                blur: Number.isFinite(value)
                  ? Math.max(0, Math.min(4, value))
                  : 0,
              })
            }}
          />
        </label>
        <label>
          {t('proofPreview.sizes')}
          <input
            className={styles.sizesInput}
            value={sizeDraft}
            aria-invalid={rejected}
            aria-describedby={sizeErrorId}
            title={t('proofPreview.sizesHint')}
            onFocus={() => {
              editingSizes.current = true
            }}
            onBlur={() => {
              editingSizes.current = false
            }}
            onChange={(event) => {
              setSizeDraft(event.target.value)
              setSizeNotice(null)
            }}
          />
        </label>
        <button type="submit">{t('proofPreview.applySizes')}</button>
      </form>
      <p
        id={sizeErrorId}
        className={rejected ? styles.error : styles.hint}
        {...(sizeNotice === null ? {} : { role: 'alert' })}
      >
        {sizeNotice === null
          ? t('proofPreview.sizesHint')
          : sizeNotice === 'truncated'
            ? t('proofPreview.sizesTruncated', {
                count: MAX_WATERFALL_STEPS,
              })
            : t(`proofPreview.sizesError.${sizeNotice}`)}
      </p>
    </>
  )
}
