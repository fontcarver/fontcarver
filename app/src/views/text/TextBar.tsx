import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import type {
  TextBarProps,
  TextDirection,
  TextSampleId,
} from './presentationTypes'
import { SpacingControls } from './SpacingControls'
import { KerningModeControl } from './KerningModeControl'
import styles from './TextBar.module.css'

export type {
  TextBarProps,
  TextDirection,
  TextFeature,
  TextMasterOption,
  TextSampleId,
} from './presentationTypes'

export const TEXT_SAMPLE_TEXT = {
  latin: 'The quick brown fox jumps over the lazy dog.',
  chinese: '天地玄黄，宇宙洪荒。',
  kerning: 'AV VA WA AW To Ta Te Ty Yo LT PA FA',
} as const

export const TEXT_SIZE_MIN = 1
export const TEXT_SIZE_MAX = 512

function parseSize(value: string): number | null {
  const trimmed = value.trim()
  if (!/^(?:\d+(?:\.\d*)?|\.\d+)$/u.test(trimmed)) return null
  const parsed = Number(trimmed)
  if (!Number.isFinite(parsed) || parsed <= 0) return null
  return Math.min(TEXT_SIZE_MAX, Math.max(TEXT_SIZE_MIN, parsed))
}

/** Stateless text-run controls with a small local draft for numeric validation. */
export function TextBar({
  text,
  onTextChange,
  inputRef,
  size,
  onSizeChange,
  masters,
  master,
  onMasterChange,
  features,
  onFeaturesChange,
  direction,
  onDirectionChange,
  onSample,
  kerningMode = 'on',
  onKerningModeChange,
  spacingSide = 'lsb',
  onSpacingSideChange,
}: TextBarProps) {
  const { t } = useTranslation()
  const [sizeDraft, setSizeDraft] = useState(String(size))
  const [sizeInvalid, setSizeInvalid] = useState(false)
  const editingSize = useRef(false)
  const suppressNextSizeBlur = useRef(false)

  useEffect(() => {
    if (!editingSize.current) setSizeDraft(String(size))
  }, [size])

  const commitSize = (value: string): boolean => {
    const next = parseSize(value)
    if (next === null) {
      setSizeDraft(String(size))
      setSizeInvalid(true)
      return false
    }
    setSizeDraft(String(next))
    setSizeInvalid(false)
    if (next !== size) onSizeChange(next)
    return true
  }

  const changeDirection = (next: TextDirection) => {
    if (next !== direction) onDirectionChange(next)
  }

  return (
    <div
      className={styles.bar}
      role="toolbar"
      aria-label={t('textControls.bar')}
    >
      <input
        ref={inputRef}
        className={styles.textInput}
        type="text"
        value={text}
        dir={direction === 'rtl' ? 'rtl' : 'ltr'}
        aria-label={t('textControls.text')}
        placeholder={t('textControls.textPlaceholder')}
        onChange={(event) => onTextChange(event.currentTarget.value)}
      />

      <select
        className={`${styles.control} ${styles.sample}`}
        value=""
        aria-label={t('textControls.sample.label')}
        onChange={(event) => {
          const sample = event.currentTarget.value as TextSampleId
          if (sample) onSample(sample)
        }}
      >
        <option value="">{t('textControls.sample.choose')}</option>
        <option value="latin">{t('textControls.sample.latin')}</option>
        <option value="chinese">{t('textControls.sample.chinese')}</option>
        <option value="all-encoded">
          {t('textControls.sample.allEncoded')}
        </option>
        <option value="kerning">{t('textControls.sample.kerning')}</option>
      </select>

      <label className={styles.sizeField}>
        <span className={styles.srOnly}>{t('textControls.size')}</span>
        <input
          className={`${styles.control} ${styles.sizeInput}`}
          type="text"
          inputMode="decimal"
          value={sizeDraft}
          aria-label={t('textControls.size')}
          aria-invalid={sizeInvalid || undefined}
          title={sizeInvalid ? t('textControls.sizeInvalid') : undefined}
          onFocus={(event) => {
            editingSize.current = true
            suppressNextSizeBlur.current = false
            event.currentTarget.select()
          }}
          onChange={(event) => {
            setSizeInvalid(false)
            setSizeDraft(event.currentTarget.value)
          }}
          onBlur={(event) => {
            editingSize.current = false
            if (suppressNextSizeBlur.current) {
              suppressNextSizeBlur.current = false
              return
            }
            commitSize(event.currentTarget.value)
          }}
          onKeyDown={(event) => {
            if (event.key === 'Enter') {
              event.preventDefault()
              editingSize.current = !commitSize(event.currentTarget.value)
            } else if (event.key === 'Escape') {
              event.preventDefault()
              editingSize.current = false
              suppressNextSizeBlur.current = true
              setSizeInvalid(false)
              setSizeDraft(String(size))
              event.currentTarget.blur()
            }
          }}
        />
        <span className={styles.unit}>{t('textControls.sizeUnit')}</span>
      </label>

      <select
        className={`${styles.control} ${styles.master}`}
        value={master}
        disabled={masters.length < 2}
        aria-label={t('textControls.master')}
        onChange={(event) => onMasterChange(event.currentTarget.value)}
      >
        {masters.map((option) => (
          <option key={option.id} value={option.id}>
            {option.name}
          </option>
        ))}
      </select>

      {onKerningModeChange && (
        <KerningModeControl mode={kerningMode} onChange={onKerningModeChange} />
      )}

      {onSpacingSideChange && (
        <SpacingControls
          side={spacingSide}
          onSideChange={onSpacingSideChange}
          disabled={kerningMode === 'locked'}
        />
      )}

      {features.length > 0 && (
        <div
          className={styles.buttonGroup}
          role="group"
          aria-label={t('textControls.features')}
        >
          {(direction === 'ttb'
            ? [
                ...features,
                ...(['vert', 'vkrn'] as const)
                  .filter((tag) => !features.some(([key]) => key === tag))
                  .map((tag) => [tag, true] as [string, boolean]),
              ]
            : features.filter(
                ([tag]) => !['vert', 'vrt2', 'vkrn'].includes(tag),
              )
          ).map(([tag, enabled], index) => (
            <button
              key={`${tag}-${index}`}
              className={styles.toggle}
              type="button"
              aria-label={tag}
              aria-pressed={
                kerningMode === 'off' && (tag === 'kern' || tag === 'vkrn')
                  ? false
                  : enabled
              }
              disabled={
                kerningMode === 'off' && (tag === 'kern' || tag === 'vkrn')
              }
              onClick={() =>
                onFeaturesChange(
                  features.some(([key]) => key === tag)
                    ? features.map(([featureTag, featureEnabled]) => [
                        featureTag,
                        featureTag === tag ? !enabled : featureEnabled,
                      ])
                    : [...features, [tag, !enabled]],
                )
              }
            >
              {tag}
            </button>
          ))}
        </div>
      )}

      <div
        className={styles.buttonGroup}
        role="group"
        aria-label={t('textControls.direction')}
      >
        <button
          className={styles.toggle}
          type="button"
          aria-label={t('textControls.directionLtr')}
          aria-pressed={direction === 'ltr'}
          onClick={() => changeDirection('ltr')}
        >
          {t('textControls.directionLtrShort')}
        </button>
        <button
          className={styles.toggle}
          type="button"
          aria-label={t('textControls.directionRtl')}
          aria-pressed={direction === 'rtl'}
          onClick={() => changeDirection('rtl')}
        >
          {t('textControls.directionRtlShort')}
        </button>
        <button
          className={styles.toggle}
          type="button"
          aria-pressed={direction === 'ttb'}
          onClick={() => changeDirection('ttb')}
          aria-label={t('textControls.directionVertical')}
          title={t('textControls.directionVertical')}
        >
          {t('textControls.directionVerticalShort')}
        </button>
      </div>
    </div>
  )
}
