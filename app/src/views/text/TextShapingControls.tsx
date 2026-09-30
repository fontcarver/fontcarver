import { useEffect, useId, useRef, useState } from 'react'
import type { ChangeEvent, FocusEvent, KeyboardEvent } from 'react'
import { useTranslation } from 'react-i18next'

import type { PositionedGlyph } from './textLayout'
import styles from './TextShapingControls.module.css'

/** Changed shaping overrides; omitted fields remain unchanged. */
export interface TextShapingChange {
  features?: [string, boolean][]
  script?: string | null
  language?: string | null
}

/** Controlled shaping settings and the current shaped output. */
export interface TextShapingControlsProps {
  features: [string, boolean][]
  script: string | null
  language: string | null
  glyphs: readonly PositionedGlyph[]
  onChange: (change: TextShapingChange) => void
}

const COMMON_FEATURES = ['kern', 'liga', 'calt', 'mark', 'mkmk', 'vert', 'vkrn']

function normalizeScript(value: string): string | null {
  return /^[A-Za-z]{4}$/u.test(value)
    ? value[0]!.toUpperCase() + value.slice(1).toLowerCase()
    : null
}

function normalizeLanguage(value: string): string | null {
  return value.length <= 255 &&
    /^[A-Za-z]{1,8}(?:-[A-Za-z0-9]{1,8})*$/u.test(value)
    ? value.toLowerCase()
    : null
}

function useOverrideDraft(
  value: string | null,
  normalize: (value: string) => string | null,
  onCommit: (value: string | null) => void,
) {
  const [draft, setDraft] = useState(value ?? '')
  const [invalid, setInvalid] = useState(false)
  const editing = useRef(false)
  const suppressBlur = useRef(false)
  useEffect(() => {
    if (!editing.current) {
      setDraft(value ?? '')
      setInvalid(false)
    }
  }, [value])

  const commit = (input: string) => {
    const trimmed = input.trim()
    const next = trimmed === '' ? null : normalize(trimmed)
    if (trimmed !== '' && next === null) {
      setInvalid(true)
      return false
    }
    setInvalid(false)
    setDraft(next ?? '')
    if (next !== value) onCommit(next)
    return true
  }

  return {
    invalid,
    input: {
      value: draft,
      'aria-invalid': invalid || undefined,
      onFocus: () => {
        editing.current = true
        suppressBlur.current = false
      },
      onChange: (event: ChangeEvent<HTMLInputElement>) => {
        setDraft(event.currentTarget.value)
        setInvalid(false)
      },
      onBlur: (event: FocusEvent<HTMLInputElement>) => {
        editing.current = false
        if (suppressBlur.current) {
          suppressBlur.current = false
          return
        }
        commit(event.currentTarget.value)
      },
      onKeyDown: (event: KeyboardEvent<HTMLInputElement>) => {
        if (event.key === 'Enter') {
          event.preventDefault()
          event.stopPropagation()
          if (commit(event.currentTarget.value)) {
            suppressBlur.current = true
            event.currentTarget.blur()
          }
        } else if (event.key === 'Escape') {
          event.preventDefault()
          event.stopPropagation()
          setDraft(value ?? '')
          setInvalid(false)
          suppressBlur.current = true
          event.currentTarget.blur()
        }
      },
    },
  }
}

/** Collapsible shaping controls and a cluster inspector in visual glyph order. */
export function TextShapingControls({
  features,
  script,
  language,
  glyphs,
  onChange,
}: TextShapingControlsProps) {
  const { t } = useTranslation()
  const id = useId()
  const [customTag, setCustomTag] = useState('')
  const [customTags, setCustomTags] = useState<string[]>([])
  const [featureInvalid, setFeatureInvalid] = useState(false)
  const scriptDraft = useOverrideDraft(script, normalizeScript, (next) =>
    onChange({ script: next }),
  )
  const languageDraft = useOverrideDraft(language, normalizeLanguage, (next) =>
    onChange({ language: next }),
  )
  const featureTags = [
    ...new Set([
      ...COMMON_FEATURES,
      ...features.map(([tag]) => tag),
      ...customTags,
    ]),
  ]
  const overrides = new Map(features)

  const addFeature = () => {
    if (!/^[\x20-\x7e]{4}$/u.test(customTag)) {
      setFeatureInvalid(true)
      return
    }
    if (!featureTags.includes(customTag))
      setCustomTags([...customTags, customTag])
    setCustomTag('')
    setFeatureInvalid(false)
  }

  return (
    <details className={styles.panel}>
      <summary className={styles.summary}>{t('proofShaping.title')}</summary>
      <div className={styles.content}>
        <div className={styles.overrides}>
          {(
            [
              ['script', scriptDraft],
              ['language', languageDraft],
            ] as const
          ).map(([name, field]) => (
            <label className={styles.override} key={name}>
              <span>{t(`proofShaping.${name}`)}</span>
              <input
                {...field.input}
                className={styles.control}
                type="text"
                spellCheck={false}
                autoCapitalize="off"
                autoComplete="off"
                placeholder={t(`proofShaping.${name}Placeholder`)}
                aria-label={t(`proofShaping.${name}`)}
                aria-describedby={
                  field.invalid ? `${id}-${name}-error` : undefined
                }
              />
              {field.invalid && (
                <span
                  className={styles.error}
                  id={`${id}-${name}-error`}
                  role="alert"
                >
                  {t(`proofShaping.${name}Invalid`)}
                </span>
              )}
            </label>
          ))}
        </div>
        <div
          className={styles.features}
          role="group"
          aria-label={t('proofShaping.features')}
        >
          {featureTags.map((tag) => (
            <label className={styles.feature} key={tag}>
              <span>{tag}</span>
              <select
                className={styles.control}
                aria-label={t('proofShaping.feature', { tag })}
                value={
                  overrides.has(tag)
                    ? overrides.get(tag)
                      ? 'on'
                      : 'off'
                    : 'default'
                }
                onChange={(event) => {
                  const value = event.currentTarget.value
                  const next = features.filter(([key]) => key !== tag)
                  if (value !== 'default') next.push([tag, value === 'on'])
                  onChange({ features: next })
                }}
              >
                <option value="default">{t('proofShaping.default')}</option>
                <option value="on">{t('proofShaping.on')}</option>
                <option value="off">{t('proofShaping.off')}</option>
              </select>
            </label>
          ))}
        </div>
        <div className={styles.customFeature}>
          <input
            className={styles.control}
            type="text"
            value={customTag}
            spellCheck={false}
            autoCapitalize="off"
            autoComplete="off"
            aria-label={t('proofShaping.customFeature')}
            placeholder={t('proofShaping.customFeature')}
            aria-invalid={featureInvalid || undefined}
            aria-describedby={
              featureInvalid ? `${id}-feature-error` : undefined
            }
            onChange={(event) => {
              setCustomTag(event.currentTarget.value)
              setFeatureInvalid(false)
            }}
            onKeyDown={(event) => {
              if (event.key === 'Enter') {
                event.preventDefault()
                event.stopPropagation()
                addFeature()
              }
            }}
          />
          <button className={styles.control} type="button" onClick={addFeature}>
            {t('proofShaping.addFeature')}
          </button>
          {featureInvalid && (
            <span
              className={styles.error}
              id={`${id}-feature-error`}
              role="alert"
            >
              {t('proofShaping.featureInvalid')}
            </span>
          )}
        </div>
        <details>
          <summary className={styles.summary}>
            {t('proofShaping.clusters', { count: glyphs.length })}
          </summary>
          <p className={styles.hint}>{t('proofShaping.clusterHint')}</p>
          {glyphs.length === 0 ? (
            <p className={styles.hint}>{t('proofShaping.noGlyphs')}</p>
          ) : (
            <div className={styles.tableScroll}>
              <table
                className={styles.table}
                aria-label={t('proofShaping.clusters', {
                  count: glyphs.length,
                })}
              >
                <thead>
                  <tr>
                    {(
                      ['sourceByte', 'glyph', 'xAdvance', 'yAdvance'] as const
                    ).map((key) => (
                      <th key={key} scope="col">
                        {t(`proofShaping.${key}`)}
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {glyphs.map((glyph, index) => (
                    <tr key={index}>
                      <td>{glyph.sourceByte}</td>
                      <td>{glyph.name ?? t('proofShaping.unnamed')}</td>
                      <td>{glyph.xAdvance}</td>
                      <td>{glyph.yAdvance}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </details>
      </div>
    </details>
  )
}
