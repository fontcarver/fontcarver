import { useEffect, useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { call, type DocSummary, type VariationState } from '../../api'
import { selectDocView, useDocumentStore } from '../../stores/documentStore'
import type { TextTabState } from '../../stores/uiStore'
import { userToDesign } from '../fontinfo/designCoordinates'
import { PreviewStrip } from './PreviewStrip'
import { kerningFeatures } from './kerningModes'
import { useVariableProof } from './useVariableProof'
import styles from './VariableProofPanel.module.css'

/** Independent read-only text proof for axis positions and named instances. */
export function VariableProofPanel({
  summary,
  state,
}: {
  summary: DocSummary
  state: TextTabState
}) {
  const { t } = useTranslation()
  const id = useId()
  const generation = useDocumentStore(selectDocView(summary.doc)).generation
  const [open, setOpen] = useState(false)
  const [metadata, setMetadata] = useState<{
    doc: number
    value: VariationState
  } | null>(null)
  const [metadataError, setMetadataError] = useState<string | null>(null)
  const [location, setLocation] = useState<Record<string, number>>({})
  const [instance, setInstance] = useState('')
  const [theme, setTheme] = useState(state.previewTheme)
  const [dismissedStatus, setDismissedStatus] = useState('')
  useEffect(() => {
    if (!open) return
    let cancelled = false
    void call('variation.state', { doc: summary.doc }).then(
      (value) => {
        if (!cancelled && value.generation === generation) {
          setMetadata({ doc: summary.doc, value })
          setMetadataError(null)
        }
      },
      (error: unknown) => {
        if (!cancelled) setMetadataError(String(error))
      },
    )
    return () => {
      cancelled = true
    }
  }, [generation, open, summary.doc])
  const model =
    metadata?.doc === summary.doc && metadata.value.generation === generation
      ? metadata.value
      : null
  const axes = (model?.axes ?? []).map((axis) => ({
    ...axis,
    tag: String.fromCharCode(...axis.tag),
  }))
  const chosen =
    instance === '' ? null : (model?.instances[Number(instance)] ?? null)
  const coordinates = Object.fromEntries(
    axes.map((axis) => {
      const tag = axis.tag
      const design = chosen?.location[tag]
      const user =
        design === undefined
          ? chosen
            ? axis.default
            : (location[tag] ?? axis.default)
          : userToDesign(
              { map: axis.map.map(([user, design]) => [design, user]) },
              design,
            )
      return [tag, Math.max(axis.min, Math.min(axis.max, user))]
    }),
  )
  const { proof, error } = useVariableProof(
    open && model
      ? {
          doc: summary.doc,
          text: state.text,
          location: coordinates,
          instance: chosen ? Number(instance) : null,
          features: kerningFeatures(state.features, state.kerningMode ?? 'on'),
          direction: state.direction,
          script: state.script ?? null,
          language: state.language ?? null,
        }
      : null,
    generation,
  )
  const statusKey = JSON.stringify([
    generation,
    coordinates,
    state.text,
    error,
    metadataError,
    !!proof,
  ])
  if (!summary.axes.length) return null
  return (
    <section className={styles.panel} aria-label={t('variableProof.title')}>
      <button
        type="button"
        className={styles.toggle}
        aria-expanded={open}
        aria-controls={id}
        onClick={() => setOpen(!open)}
      >
        {t('variableProof.title')}
      </button>
      {open && (
        <div id={id}>
          <div className={styles.controls}>
            <label>
              {t('variableProof.instance')}
              <select
                value={chosen ? instance : ''}
                disabled={!model}
                onChange={(event) => setInstance(event.target.value)}
              >
                <option value="">{t('variableProof.custom')}</option>
                {model?.instances.map((value, index) => (
                  <option key={index} value={index}>
                    {value.name}
                  </option>
                ))}
              </select>
            </label>
            <button
              type="button"
              onClick={() => {
                setInstance('')
                setLocation({})
              }}
            >
              {t('variableProof.reset')}
            </button>
            {axes
              .filter((axis) => !axis.hidden)
              .map((axis) => (
                <label key={axis.tag}>
                  <span>{axis.name || axis.tag}</span>
                  <input
                    type="range"
                    min={axis.min}
                    max={axis.max}
                    step={(axis.max - axis.min) / 1000 || 1}
                    value={coordinates[axis.tag] ?? axis.default}
                    onChange={(event) => {
                      setLocation({
                        ...coordinates,
                        [axis.tag]: Number(event.target.value),
                      })
                      setInstance('')
                    }}
                  />
                  <output>
                    {Number((coordinates[axis.tag] ?? axis.default).toFixed(2))}
                  </output>
                </label>
              ))}
          </div>
          {(error || metadataError) && (
            <p className={styles.error} role="alert">
              {t('variableProof.failed', { message: error || metadataError })}
            </p>
          )}
          {!!proof?.warnings.length && (
            <p role="status">{proof.warnings.join('; ')}</p>
          )}
          <PreviewStrip
            glyphs={(proof?.glyphs ?? []).map((glyph) => ({
              path: glyph.path,
              x: glyph.x,
              y: glyph.y,
              xAdvance: glyph.x_advance,
              yAdvance: glyph.y_advance,
            }))}
            unitsPerEm={proof?.units_per_em ?? summary.units_per_em}
            status={
              dismissedStatus === statusKey
                ? 'ready'
                : error || metadataError
                  ? 'compile-failed'
                  : proof
                    ? 'ready'
                    : 'stale'
            }
            onDismissStatus={() => setDismissedStatus(statusKey)}
            theme={theme}
            onThemeChange={setTheme}
            collapsed={false}
            onCollapsedChange={(collapsed) => setOpen(!collapsed)}
          />
        </div>
      )}
    </section>
  )
}
