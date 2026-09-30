import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { VariationPreview } from '../../api'
import { Button } from '../../components/Button'
import { parseSourcePoints, samePointStructure } from './variationCompatibility'
import styles from './VariationCompatibility.module.css'

/** Inspect source outlines and their ordered points before invoking compatibility repair. */
export function VariationCompatibility({
  preview,
  busy,
  glyph,
  onRepair,
}: {
  preview: VariationPreview | null
  busy: boolean
  glyph: string
  onRepair: () => void
}) {
  const { t } = useTranslation()
  const [showSources, setShowSources] = useState(true)
  const [showPoints, setShowPoints] = useState(false)
  const [hidden, setHidden] = useState<Set<string>>(() => new Set())
  const sources = useMemo(
    () =>
      preview?.sources.map((source, index) => ({
        ...source,
        index,
        id: `${glyph}:${index}:${source.name}`,
        points: parseSourcePoints(source.path),
      })) ?? [],
    [preview?.sources, glyph],
  )
  const selected = sources.filter((source) => !hidden.has(source.id))
  const reference = selected[0]?.points
  const pointsAvailable =
    selected.every((source) => source.points !== null) &&
    selected.reduce(
      (sum, source) => sum + (source.points?.pointCount ?? 0),
      0,
    ) <= 750
  const matches =
    reference &&
    selected.every(
      (source) => source.points && samePointStructure(reference, source.points),
    )
  const pointMessage = !pointsAvailable
    ? 'pointLimit'
    : selected.length < 2
      ? 'selectSources'
      : !matches
        ? 'mismatch'
        : null
  const em = preview && preview.units_per_em > 0 ? preview.units_per_em : 1000
  const pointList = sources.flatMap(
    (source) =>
      source.points?.contours.flatMap((contour) => contour.points) ?? [],
  )
  const left = Math.min(-em * 0.15, ...pointList.map((point) => point.x))
  const right = Math.max(
    em,
    preview?.width ?? 0,
    ...pointList.map((point) => point.x),
  )
  const bottom = Math.min(-em * 0.2, ...pointList.map((point) => point.y))
  const top = Math.max(em, ...pointList.map((point) => point.y))
  const margin = em * 0.05
  const pointSize = Math.max(right - left, top - bottom) / 180
  const sourceClass = (index: number) => styles[`source${index % 6}`]

  return (
    <section
      className={styles.panel}
      aria-label={t('variation.compatibility.title')}
    >
      <svg
        viewBox={`${left - margin} ${-top - margin} ${right - left + 2 * margin} ${top - bottom + 2 * margin}`}
        className={styles.canvas}
        aria-label={t('variation.outline')}
        role="img"
        aria-busy={busy}
      >
        <g transform="scale(1,-1)">
          <path d={`M${left} 0H${right}`} className={styles.baseline} />
          <path
            d={preview?.path ?? ''}
            className={`${styles.outline} ${showSources ? styles.faded : ''}`}
          />
          {showSources &&
            selected.map((source) => (
              <path
                key={source.id}
                data-source-outline={source.name}
                d={source.path}
                className={`${styles.source} ${sourceClass(source.index)}`}
              >
                <title>{source.name}</title>
              </path>
            ))}
          {showSources &&
            showPoints &&
            pointsAvailable &&
            matches &&
            selected.length > 1 &&
            reference.contours.map((contour, ci) =>
              contour.points.map((point, pi) =>
                selected.slice(1).map((source) => {
                  const other = source.points!.contours[ci]!.points[pi]!
                  return (
                    <line
                      key={`${source.id}:${ci}:${pi}`}
                      data-correspondence=""
                      className={styles.correspondence}
                      x1={point.x}
                      y1={point.y}
                      x2={other.x}
                      y2={other.y}
                    />
                  )
                }),
              ),
            )}
          {showSources &&
            showPoints &&
            pointsAvailable &&
            selected.map((source) =>
              source.points?.contours.map((contour, ci) =>
                contour.points.map((point, pi) => (
                  <g
                    key={`${source.id}:${ci}:${pi}`}
                    data-point-marker=""
                    className={sourceClass(source.index)}
                    transform={`translate(${point.x},${point.y}) scale(1,-1)`}
                  >
                    <title>
                      {t('variation.compatibility.point', {
                        source: source.name,
                        contour: ci + 1,
                        point: pi + 1,
                      })}
                    </title>
                    <circle
                      r={point.kind === 'on' ? pointSize : pointSize * 0.75}
                      className={
                        point.kind === 'on' ? styles.onCurve : styles.control
                      }
                    />
                    <text
                      x={pointSize * 1.6}
                      y={-pointSize * 1.4}
                      fontSize={pointSize * 3.3}
                      className={styles.pointLabel}
                    >{`${ci + 1}.${pi + 1}`}</text>
                  </g>
                )),
              ),
            )}
        </g>
      </svg>
      <div className={styles.toolbar}>
        <label>
          <input
            type="checkbox"
            checked={showSources}
            onChange={(event) => setShowSources(event.target.checked)}
          />
          {t('variation.showSources')}
        </label>
        <label>
          <input
            type="checkbox"
            checked={showPoints}
            disabled={!showSources || !sources.length}
            onChange={(event) => setShowPoints(event.target.checked)}
          />
          {t('variation.compatibility.points')}
        </label>
        <span>
          {t('variation.advance', { value: preview?.width.toFixed(1) ?? '—' })}
        </span>
      </div>
      <fieldset className={styles.legend} disabled={!showSources}>
        <legend>{t('variation.compatibility.title')}</legend>
        {sources.map((source) => (
          <label key={source.id} className={styles.sourceRow}>
            <input
              type="checkbox"
              aria-label={t('variation.compatibility.source', {
                name: source.name,
              })}
              checked={!hidden.has(source.id)}
              onChange={(event) =>
                setHidden((previous) => {
                  const next = new Set(previous)
                  if (event.target.checked) next.delete(source.id)
                  else next.add(source.id)
                  return next
                })
              }
            />
            <span
              className={`${styles.swatch} ${sourceClass(source.index)}`}
              aria-hidden="true"
            />
            <span className={styles.sourceName}>{source.name}</span>
            <small>
              {source.points
                ? t('variation.compatibility.summary', {
                    contours: source.points.contours.length,
                    points: source.points.pointCount,
                  })
                : t('variation.compatibility.pointLimit')}
            </small>
            {showPoints && selected[0]?.id === source.id && (
              <small>{t('variation.compatibility.reference')}</small>
            )}
          </label>
        ))}
      </fieldset>
      {sources.length >= 16 && (
        <p className={styles.guidance}>
          {t('variation.compatibility.sourceLimit')}
        </p>
      )}
      {showSources && showPoints && (
        <div className={styles.guidance}>
          <p>{t('variation.compatibility.pointHint')}</p>
          {pointMessage && (
            <p role="status">{t(`variation.compatibility.${pointMessage}`)}</p>
          )}
        </div>
      )}
      {preview?.issues.length ? (
        <div className={styles.issues}>
          <h4>{t('variation.compatibility.issues')}</h4>
          <ul>
            {preview.issues.map((issue, i) => (
              <li key={i}>{issue}</li>
            ))}
          </ul>
        </div>
      ) : (
        <p className={styles.status}>
          {t(
            preview
              ? 'variation.compatibility.ready'
              : 'variation.compatibility.pending',
          )}
        </p>
      )}
      <Button disabled={busy || !glyph || !preview} onClick={onRepair}>
        {t('variation.repair')}
      </Button>
      <p className={styles.guidance}>{t('variation.repairHint')}</p>
    </section>
  )
}
