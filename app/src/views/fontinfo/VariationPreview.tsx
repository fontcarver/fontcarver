import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { call, type DocId, type VariationPreview as Preview } from '../../api'
import { IconButton } from '../../components/IconButton'
import { NumberField } from '../../components/NumberField'
import { axisStep, formatAxisValue } from './designCoordinates'
import type { DesignSpaceValue as VariationState } from './designSpaceSession'
import { VariationCompatibility } from './VariationCompatibility'
import { errorNotice } from '../../i18n/errors'
import styles from './designSpace.module.css'

/** Convert mapped design coordinates from a named instance back to slider units. */
function userValue(
  axis: VariationState['axes'][number],
  value: number,
): number {
  const map = axis.map
  if (!map.length) return value
  const first = map[0]!,
    last = map[map.length - 1]!
  if (value <= first[1]) return value + first[0] - first[1]
  for (let i = 1; i < map.length; i++) {
    const a = map[i - 1]!,
      b = map[i]!
    if (value <= b[1])
      return a[0] + ((value - a[1]) / (b[1] - a[1])) * (b[0] - a[0])
  }
  return value + last[0] - last[1]
}

export function VariationPreview({
  doc,
  state,
  onRefresh,
  fixedGlyph,
  onPreviewChange,
}: {
  doc: DocId
  state: VariationState
  onRefresh: () => void
  fixedGlyph?: string
  onPreviewChange?: (preview: Preview | null) => void
}) {
  const { t } = useTranslation()
  const [glyph, setGlyph] = useState(fixedGlyph ?? state.first_glyph ?? '')
  const [location, setLocation] = useState<Record<string, number>>({})
  const requestKey = JSON.stringify([doc, glyph, location, state.generation])
  const [response, setResponse] = useState<{
    key: string
    value: Preview
  } | null>(null)
  const [settledKey, setSettledKey] = useState<string | null>(null)
  const preview = response?.key === requestKey ? response.value : null
  const [onCanvas, setOnCanvas] = useState(true)
  useEffect(() => {
    onPreviewChange?.(
      onCanvas && (preview?.path || !preview?.issues.length) ? preview : null,
    )
    return () => onPreviewChange?.(null)
  }, [onCanvas, preview, onPreviewChange])
  const [error, setError] = useState(''),
    [busy, setBusy] = useState(false)
  useEffect(() => {
    let current = true
    const timer = setTimeout(() => {
      setBusy(true)
      setError('')
      void call('variation.preview', { doc, glyph, location })
        .then((result) => {
          if (current) {
            setResponse({ key: requestKey, value: result })
            setError('')
          }
        })
        .catch((error: unknown) => {
          if (current) {
            setResponse(null)
            setError(
              errorNotice(error, t).details ?? errorNotice(error, t).message,
            )
          }
        })
        .finally(() => {
          if (current) {
            setSettledKey(requestKey)
            setBusy(false)
          }
        })
    }, 60)
    return () => {
      current = false
      clearTimeout(timer)
    }
  }, [doc, glyph, location, requestKey, t])
  const repair = () => {
    setBusy(true)
    void call('variation.apply', {
      doc,
      expected_generation: state.generation,
      command: { Repair: { glyphs: [glyph] } },
      label: t('variation.repair'),
    })
      .then(onRefresh)
      .catch((error: unknown) =>
        setError(
          errorNotice(error, t).details ?? errorNotice(error, t).message,
        ),
      )
      .finally(() => setBusy(false))
  }
  /** One swap of shape order in one source: one command, one undo entry. */
  const reorder = (layer: string, order: number[]) => {
    setBusy(true)
    void call('variation.apply', {
      doc,
      expected_generation: state.generation,
      command: { ReorderShapes: { glyph, layer, order } },
      label: t('variation.reorderShapes'),
    })
      .then(onRefresh)
      .catch((error: unknown) =>
        setError(
          errorNotice(error, t).details ?? errorNotice(error, t).message,
        ),
      )
      .finally(() => setBusy(false))
  }
  return (
    <section className={styles.preview} aria-label={t('variation.preview')}>
      <h3>{t('variation.preview')}</h3>
      <div className={styles.row}>
        <label>
          {t('variation.glyph')}
          <input
            value={glyph}
            readOnly={fixedGlyph !== undefined}
            onChange={(e) => setGlyph(e.target.value)}
          />
        </label>
        <label>
          {t('variation.namedInstance')}
          <select
            defaultValue=""
            onChange={(e) => {
              const instance = state.instances[Number(e.target.value)]
              if (instance)
                setLocation(
                  Object.fromEntries(
                    state.axes.map((a) => [
                      a.tag,
                      instance.location[a.tag] === undefined
                        ? a.default
                        : userValue(a, instance.location[a.tag]!),
                    ]),
                  ),
                )
            }}
          >
            <option value="" disabled>
              {t('variation.chooseInstance')}
            </option>
            {state.instances.map((instance, i) => (
              <option key={i} value={i}>
                {instance.name}
              </option>
            ))}
          </select>
        </label>
      </div>
      {state.axes.map((axis) => {
        const step = axisStep(
          axis,
          state.instances
            .map((instance) => instance.location[axis.tag])
            .filter((value): value is number => value !== undefined)
            .map((value) => userValue(axis, value)),
        )
        const value = location[axis.tag] ?? axis.default
        const set = (next: number) =>
          setLocation({
            ...location,
            [axis.tag]: Math.min(axis.max, Math.max(axis.min, next)),
          })
        return (
          <div className={styles.slider} key={axis.tag}>
            {/* The field carries the axis name, so the scrubber names its job. */}
            <NumberField
              label={axis.name}
              value={value}
              step={step}
              onCommit={set}
              formatValue={(shown) => formatAxisValue(shown, step)}
            />
            <input
              type="range"
              aria-label={t('variation.scrubAxis', { axis: axis.name })}
              min={axis.min}
              max={axis.max}
              step={step}
              value={value}
              onChange={(e) => set(Number(e.target.value))}
            />
            <IconButton
              label={t('variation.resetAxis', { axis: axis.name })}
              disabled={value === axis.default}
              onClick={() => set(axis.default)}
            >
              ↺
            </IconButton>
          </div>
        )
      })}
      {onPreviewChange && (
        <label>
          <input
            type="checkbox"
            checked={onCanvas}
            onChange={(e) => setOnCanvas(e.target.checked)}
          />
          {t('variation.showOnCanvas')}
        </label>
      )}
      {error && <p role="alert">{error}</p>}
      <VariationCompatibility
        key={`${doc}:${glyph}`}
        preview={preview}
        busy={busy || settledKey !== requestKey}
        glyph={glyph}
        onRepair={repair}
        onReorder={reorder}
      />
    </section>
  )
}
