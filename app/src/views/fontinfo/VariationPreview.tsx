import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { call, type DocId, type VariationPreview as Preview } from '../../api'
import type { DesignSpaceValue as VariationState } from './designSpaceSession'
import { Button } from '../../components/Button'
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
}: {
  doc: DocId
  state: VariationState
  onRefresh: () => void
  fixedGlyph?: string
}) {
  const { t } = useTranslation()
  const [glyph, setGlyph] = useState(fixedGlyph ?? state.first_glyph ?? '')
  const [location, setLocation] = useState<Record<string, number>>({})
  const [preview, setPreview] = useState<Preview | null>(null)
  const [error, setError] = useState(''),
    [busy, setBusy] = useState(false),
    [overlays, setOverlays] = useState(true)
  useEffect(() => {
    let current = true
    const timer = setTimeout(() => {
      setBusy(true)
      void call('variation.preview', { doc, glyph, location })
        .then((result) => {
          if (current) {
            setPreview(result)
            setError('')
          }
        })
        .catch((error: unknown) => {
          if (current) {
            setPreview(null)
            setError(
              errorNotice(error, t).details ?? errorNotice(error, t).message,
            )
          }
        })
        .finally(() => {
          if (current) setBusy(false)
        })
    }, 60)
    return () => {
      current = false
      clearTimeout(timer)
    }
  }, [doc, glyph, location, state.generation, t])
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
  const em = preview?.units_per_em ?? 1000
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
      {state.axes.map((axis) => (
        <label className={styles.slider} key={axis.tag}>
          {axis.name}
          <input
            type="range"
            aria-label={axis.name}
            min={axis.min}
            max={axis.max}
            step={(axis.max - axis.min) / 1000}
            value={location[axis.tag] ?? axis.default}
            onChange={(e) =>
              setLocation({ ...location, [axis.tag]: Number(e.target.value) })
            }
          />
          <output>{(location[axis.tag] ?? axis.default).toFixed(1)}</output>
        </label>
      ))}
      <svg
        viewBox={`${-em * 0.15} ${-em} ${Math.max(em, preview?.width ?? 0) * 1.3} ${em * 1.25}`}
        className={styles.canvas}
        aria-label={t('variation.outline')}
        role="img"
        aria-busy={busy}
      >
        <g transform="scale(1,-1)">
          <path d={`M${-em * 0.1} 0H${em * 1.2}`} className={styles.baseline} />
          {overlays &&
            preview?.sources.map((source, index) => (
              <path
                key={index}
                d={source.path}
                className={`${styles.source} ${styles[`source${index % 3}`]}`}
              >
                <title>{source.name}</title>
              </path>
            ))}
          <path d={preview?.path ?? ''} className={styles.outline} />
        </g>
      </svg>
      <div className={styles.row}>
        <label>
          <input
            type="checkbox"
            checked={overlays}
            onChange={(e) => setOverlays(e.target.checked)}
          />
          {t('variation.showSources')}
        </label>
        <span>
          {t('variation.advance', { value: preview?.width.toFixed(1) ?? '—' })}
        </span>
      </div>
      {error && <p role="alert">{error}</p>}
      {preview && preview.issues.length > 0 && (
        <ul>
          {preview.issues.map((issue, i) => (
            <li key={i}>{issue}</li>
          ))}
        </ul>
      )}
      <Button disabled={busy || !glyph} onClick={repair}>
        {t('variation.repair')}
      </Button>
      <p>{t('variation.repairHint')}</p>
    </section>
  )
}
