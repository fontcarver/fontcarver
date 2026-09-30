import { useEffect, useId, useState } from 'react'
import { useTranslation } from 'react-i18next'

import {
  call,
  type DocId,
  type EditorSettingsView,
  type FontEditorSettings,
} from '../../api'
import { Button } from '../../components/Button'
import { NumberField } from '../../components/NumberField'
import { withSettledDocument } from '../../editor/documentPersistence'
import { errorNotice } from '../../i18n/errors'
import { invalidateFontInfo } from '../../stores/fontInfoCache'

import styles from './fontInfo.module.css'

/** The three arrow-key steps, in the order the kernel stores them. */
const STEPS = ['plain', 'shift', 'command'] as const

/**
 * Per-font grid spacing, grid subdivision, keyboard increments and the
 * units-per-em change that can scale the drawing with it.
 *
 * These belong to the drawing rather than to the person, so they live in the
 * project: a pixel font needs a grid of two, a text face needs one. A field
 * left blank inherits the application preference, and the effective value is
 * shown beside it.
 */
export function FontSettingsEditor({
  doc,
  unitsPerEm,
}: {
  doc: DocId
  unitsPerEm: number
}) {
  const { t } = useTranslation()
  const id = useId()
  const [view, setView] = useState<EditorSettingsView | null>(null)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState<string[]>([])
  const [busy, setBusy] = useState(false)
  const [scale, setScale] = useState({
    outlines: true,
    metrics: true,
    kerning: true,
    guides: true,
  })
  // The field follows the document until the user types a different number,
  // which is a derived value rather than an effect.
  const [typed, setTyped] = useState<{ source: number; value: number } | null>(
    null,
  )
  const upm = typed?.source === unitsPerEm ? typed.value : unitsPerEm
  const setUpm = (value: number) => setTyped({ source: unitsPerEm, value })
  useEffect(() => {
    let current = true
    void call('fontinfo.editor_settings_get', { doc })
      .then((result) => {
        if (current) setView(result)
      })
      .catch((cause: unknown) => {
        if (current) setError(errorNotice(cause, t).message)
      })
    return () => {
      current = false
    }
  }, [doc, t])

  const store = (settings: FontEditorSettings) => {
    setBusy(true)
    setError('')
    void call('fontinfo.editor_settings_set', { doc, settings })
      .then((result) => {
        setView(result)
        invalidateFontInfo(doc)
      })
      .catch((cause: unknown) => {
        const failure = errorNotice(cause, t)
        setError(failure.details ?? failure.message)
      })
      .finally(() => setBusy(false))
  }

  const stored = view?.stored ?? {}
  const rescale = () => {
    setBusy(true)
    setError('')
    setNotice([])
    void withSettledDocument(doc, () =>
      call('variation.state', { doc }).then((state) =>
        call('variation.rescale_upm', {
          doc,
          expected_generation: state.generation,
          units_per_em: upm,
          scale_outlines: scale.outlines,
          scale_metrics: scale.metrics,
          scale_kerning: scale.kerning,
          scale_guides: scale.guides,
          label: t('fontSettings.rescaleLabel'),
        }),
      ),
    )
      .then((result) => {
        invalidateFontInfo(doc)
        setNotice([
          t('fontSettings.rescaled', {
            factor: result.factor.toFixed(4),
            glyphs: result.glyphs,
            layers: result.layers,
          }),
          ...(result.cleared_programs > 0
            ? [
                t('fontSettings.clearedPrograms', {
                  count: result.cleared_programs,
                }),
              ]
            : []),
          ...result.warnings,
        ])
      })
      .catch((cause: unknown) => {
        const failure = errorNotice(cause, t)
        setError(failure.details ?? failure.message)
      })
      .finally(() => setBusy(false))
  }

  return (
    <section aria-labelledby={`${id}-heading`}>
      <h3 id={`${id}-heading`}>{t('fontSettings.title')}</h3>
      <p className={styles.scopeHint}>{t('fontSettings.hint')}</p>
      <fieldset disabled={busy || view === null}>
        <legend>{t('fontSettings.grid')}</legend>
        <div className={styles.scopeEntries}>
          <label>
            <input
              type="checkbox"
              checked={stored.grid_size !== undefined}
              onChange={(event) =>
                store({
                  ...stored,
                  grid_size: event.target.checked
                    ? (view?.grid_size ?? 1)
                    : undefined,
                })
              }
            />
            {t('fontSettings.overrideGrid')}
          </label>
          <NumberField
            label={t('fontSettings.gridSize')}
            value={view?.grid_size ?? 1}
            disabled={stored.grid_size === undefined}
            onCommit={(value) => store({ ...stored, grid_size: value })}
          />
          <span className={styles.scopeHint}>
            {t(
              stored.grid_size === undefined
                ? 'fontSettings.inherited'
                : 'fontSettings.perFont',
              { value: view?.grid_size ?? 1 },
            )}
          </span>
        </div>
        <div className={styles.scopeEntries}>
          <label>
            <input
              type="checkbox"
              checked={stored.grid_subdivision !== undefined}
              onChange={(event) =>
                store({
                  ...stored,
                  grid_subdivision: event.target.checked
                    ? (view?.grid_subdivision ?? 1)
                    : undefined,
                })
              }
            />
            {t('fontSettings.overrideSubdivision')}
          </label>
          <NumberField
            label={t('fontSettings.subdivision')}
            value={view?.grid_subdivision ?? 1}
            disabled={stored.grid_subdivision === undefined}
            onCommit={(value) =>
              store({ ...stored, grid_subdivision: Math.round(value) })
            }
          />
          <span className={styles.scopeHint}>
            {t(
              stored.grid_subdivision === undefined
                ? 'fontSettings.inherited'
                : 'fontSettings.perFont',
              { value: view?.grid_subdivision ?? 1 },
            )}
          </span>
        </div>
      </fieldset>
      <fieldset disabled={busy || view === null}>
        <legend>{t('fontSettings.increments')}</legend>
        <label>
          <input
            type="checkbox"
            checked={stored.nudge !== undefined}
            onChange={(event) =>
              store({
                ...stored,
                nudge: event.target.checked
                  ? (view?.nudge ?? [1, 10, 100])
                  : undefined,
              })
            }
          />
          {t('fontSettings.overrideIncrements')}
        </label>
        <div className={styles.scopeEntries}>
          {STEPS.map((step, index) => (
            <NumberField
              key={step}
              label={t(`fontSettings.step.${step}`)}
              value={view?.nudge[index] ?? 1}
              disabled={stored.nudge === undefined}
              onCommit={(value) => {
                const next: [number, number, number] = [
                  ...(view?.nudge ?? [1, 10, 100]),
                ] as [number, number, number]
                next[index] = value
                store({ ...stored, nudge: next })
              }}
            />
          ))}
        </div>
        <span className={styles.scopeHint}>
          {t(
            stored.nudge === undefined
              ? 'fontSettings.inherited'
              : 'fontSettings.perFont',
            { value: (view?.nudge ?? []).join(', ') },
          )}
        </span>
      </fieldset>
      <fieldset disabled={busy}>
        <legend>{t('fontSettings.upm')}</legend>
        <p className={styles.scopeHint}>{t('fontSettings.upmHint')}</p>
        <NumberField
          label={t('fontSettings.upmValue')}
          value={upm}
          onCommit={(value) => setUpm(Math.round(value))}
        />
        {(
          [
            ['outlines', 'fontSettings.scaleOutlines'],
            ['metrics', 'fontSettings.scaleMetrics'],
            ['kerning', 'fontSettings.scaleKerning'],
            ['guides', 'fontSettings.scaleGuides'],
          ] as const
        ).map(([field, label]) => (
          <label key={field}>
            <input
              type="checkbox"
              checked={scale[field]}
              onChange={(event) =>
                setScale((current) => ({
                  ...current,
                  [field]: event.target.checked,
                }))
              }
            />
            {t(label)}
          </label>
        ))}
        <Button
          variant="primary"
          disabled={busy || upm === unitsPerEm}
          onClick={rescale}
        >
          {t('fontSettings.apply')}
        </Button>
      </fieldset>
      {notice.map((line) => (
        <p key={line} role="status">
          {line}
        </p>
      ))}
      {error && (
        <p role="alert" className={styles.empty}>
          {error}
        </p>
      )}
    </section>
  )
}
