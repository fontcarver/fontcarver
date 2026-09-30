import { useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { VariationPreview } from '../../api'
import { Button } from '../../components/Button'
import {
  nudgeOrder,
  shapeColour,
  shapeSegments,
  swapOrder,
} from './variationCompatibility'
import styles from './VariationCompatibility.module.css'

/** Largest number of stored points the view draws markers and chords for. */
const MAX_MARKERS = 1200

/**
 * The master-compatibility view.
 *
 * Shapes are coloured and numbered by shape order, which is the interpolation
 * contract, so the same colour is the same shape in every source. Connectors
 * join corresponding start points and shape centres: crossing connectors are
 * exactly the "shape shifter" the drag-to-swap repair fixes. Segment colours
 * are the kernel's per-segment status — green compatible, yellow a direction
 * difference above the warning angle, red incompatible.
 */
export function VariationCompatibility({
  preview,
  busy,
  glyph,
  onRepair,
  onReorder,
}: {
  preview: VariationPreview | null
  busy: boolean
  glyph: string
  onRepair: () => void
  onReorder?: (layer: string, order: number[]) => void
}) {
  const { t } = useTranslation()
  const [showSources, setShowSources] = useState(true)
  const [showPoints, setShowPoints] = useState(false)
  const [hidden, setHidden] = useState<Set<string>>(() => new Set())
  const [selectedSource, setSelectedSource] = useState(0)
  const [drag, setDrag] = useState<{
    from: number
    x: number
    y: number
  } | null>(null)
  const canvas = useRef<SVGSVGElement | null>(null)
  const sources = useMemo(
    () =>
      preview?.sources.map((source, index) => ({
        ...source,
        sourceIndex: index,
        id: `${glyph}:${index}:${source.name}`,
      })) ?? [],
    [preview?.sources, glyph],
  )
  const selected = sources.filter((source) => !hidden.has(source.id))
  const reference = selected[0]
  const status = preview?.shape_status ?? []
  const shapeCount = reference?.shapes.length ?? 0
  const markerCount = selected.reduce(
    (sum, source) =>
      sum +
      source.shapes.reduce((count, shape) => count + shape.points.length, 0),
    0,
  )
  const markersAvailable = markerCount > 0 && markerCount <= MAX_MARKERS
  const em = preview && preview.units_per_em > 0 ? preview.units_per_em : 1000
  const allPoints = selected.flatMap((source) =>
    source.shapes.flatMap((shape) => shape.points),
  )
  const left = Math.min(-em * 0.15, ...allPoints.map((point) => point[0]))
  const right = Math.max(
    em,
    preview?.width ?? 0,
    ...allPoints.map((point) => point[0]),
  )
  const bottom = Math.min(-em * 0.2, ...allPoints.map((point) => point[1]))
  const top = Math.max(em, ...allPoints.map((point) => point[1]))
  const margin = em * 0.05
  const size = Math.max(right - left, top - bottom)
  const pointSize = size / 180
  const shapeClass = (position: number) =>
    styles[`shape${shapeColour(position)}`]
  const statusClass = (value: string) =>
    value === 'incompatible'
      ? styles.segmentIncompatible
      : value === 'angle'
        ? styles.segmentWarning
        : styles.segmentCompatible

  /** Reorder within the source the legend has selected. */
  const target = sources[selectedSource]
  const reorder = (order: number[]) => {
    if (!onReorder || !target || busy) return
    if (order.every((value, index) => value === index)) return
    onReorder(target.layer, order)
  }

  /** Font-space position of a pointer event inside the canvas. */
  const fontPoint = (event: React.PointerEvent): { x: number; y: number } => {
    const element = canvas.current
    if (!element) return { x: 0, y: 0 }
    const box = element.getBoundingClientRect()
    const width = right - left + 2 * margin
    const height = top - bottom + 2 * margin
    // The viewBox is uniformly fitted by `preserveAspectRatio`'s default.
    const scale = Math.min(box.width / width, box.height / height) || 1
    const originX = box.left + (box.width - width * scale) / 2
    const originY = box.top + (box.height - height * scale) / 2
    return {
      x: left - margin + (event.clientX - originX) / scale,
      y: -(-top - margin + (event.clientY - originY) / scale),
    }
  }

  /** Shape whose centre is nearest a font-space point, within a tolerance. */
  const nearestShape = (point: { x: number; y: number }): number | null => {
    if (!target) return null
    const distances = target.shapes.map((shape, index) => ({
      index,
      distance: Math.hypot(
        shape.center[0] - point.x,
        shape.center[1] - point.y,
      ),
    }))
    const best = distances.reduce<{ index: number; distance: number } | null>(
      (found, candidate) =>
        found === null || candidate.distance < found.distance
          ? candidate
          : found,
      null,
    )
    return best !== null && best.distance <= size * 0.5 ? best.index : null
  }

  return (
    <section
      className={styles.panel}
      aria-label={t('variation.compatibility.title')}
    >
      <svg
        ref={canvas}
        viewBox={`${left - margin} ${-top - margin} ${right - left + 2 * margin} ${top - bottom + 2 * margin}`}
        className={styles.canvas}
        aria-label={t('variation.outline')}
        role="img"
        aria-busy={busy}
        onPointerMove={(event) => {
          if (!drag) return
          const point = fontPoint(event)
          setDrag({ ...drag, x: point.x, y: point.y })
        }}
        onPointerUp={(event) => {
          if (!drag) return
          const point = fontPoint(event)
          const to = nearestShape(point)
          setDrag(null)
          if (to !== null && to !== drag.from)
            reorder(swapOrder(shapeCount, drag.from, to))
        }}
        onPointerCancel={() => setDrag(null)}
      >
        <defs>
          <pattern
            id={`fc-component-${glyph}`}
            width="8"
            height="8"
            patternUnits="userSpaceOnUse"
          >
            <rect width="4" height="4" className={styles.checker} />
            <rect x="4" y="4" width="4" height="4" className={styles.checker} />
          </pattern>
        </defs>
        <g transform="scale(1,-1)">
          <path d={`M${left} 0H${right}`} className={styles.baseline} />
          <path
            d={preview?.path ?? ''}
            className={`${styles.outline} ${showSources ? styles.faded : ''}`}
          />
          {showSources &&
            selected.map((source) =>
              source.shapes.map((shape, position) => (
                <path
                  key={`${source.id}:${position}`}
                  data-source-outline={source.name}
                  data-shape={position}
                  d={shape.path}
                  className={`${styles.source} ${shapeClass(position)} ${
                    source.sourceIndex === reference?.sourceIndex
                      ? ''
                      : styles.secondary
                  } ${shape.component ? styles.component : ''}`}
                  style={
                    shape.component
                      ? { fill: `url(#fc-component-${glyph})` }
                      : undefined
                  }
                >
                  <title>
                    {shape.component
                      ? t('variation.compatibility.componentShape', {
                          source: source.name,
                          index: position + 1,
                          base: shape.component,
                        })
                      : t('variation.compatibility.shape', {
                          source: source.name,
                          index: position + 1,
                        })}
                  </title>
                </path>
              )),
            )}
          {/* Connectors between corresponding start points and shape centres. */}
          {showSources &&
            selected.length > 1 &&
            Array.from({ length: shapeCount }, (_, position) => {
              const starts = selected
                .map((source) => source.shapes[position])
                .filter((shape) => shape !== undefined)
              if (starts.length < 2) return null
              const line = (key: 'start' | 'center') =>
                starts
                  .map((shape) => `${shape![key][0]},${shape![key][1]}`)
                  .join(' ')
              return (
                <g
                  key={`connector-${position}`}
                  className={shapeClass(position)}
                >
                  <polyline
                    data-connector="start"
                    points={line('start')}
                    className={styles.startConnector}
                  />
                  <polyline
                    data-connector="center"
                    points={line('center')}
                    className={styles.centerConnector}
                  />
                </g>
              )
            })}
          {/* Per-segment status on the reference source. */}
          {showSources &&
            markersAvailable &&
            reference?.shapes.map((shape, position) =>
              shapeSegments(shape, status[position]).map((segment, index) => (
                <line
                  key={`segment-${position}-${index}`}
                  data-segment-status={segment.status}
                  className={statusClass(segment.status)}
                  x1={segment.from.x}
                  y1={segment.from.y}
                  x2={segment.to.x}
                  y2={segment.to.y}
                />
              )),
            )}
          {showSources &&
            showPoints &&
            markersAvailable &&
            selected.map((source) =>
              source.shapes.map((shape, position) =>
                shape.points.map((point, index) => (
                  <g
                    key={`${source.id}:${position}:${index}`}
                    data-point-marker=""
                    className={shapeClass(position)}
                    transform={`translate(${point[0]},${point[1]}) scale(1,-1)`}
                  >
                    <title>
                      {t('variation.compatibility.point', {
                        source: source.name,
                        contour: position + 1,
                        point: index + 1,
                      })}
                    </title>
                    <circle
                      r={shape.on_curve[index] ? pointSize : pointSize * 0.75}
                      className={
                        shape.on_curve[index] ? styles.onCurve : styles.control
                      }
                    />
                  </g>
                )),
              ),
            )}
          {/* Shape-order index at each start node of the reference source. */}
          {showSources &&
            reference?.shapes.map((shape, position) => (
              <g
                key={`index-${position}`}
                className={shapeClass(position)}
                transform={`translate(${shape.start[0]},${shape.start[1]}) scale(1,-1)`}
              >
                <text
                  x={pointSize * 1.8}
                  y={-pointSize * 1.6}
                  fontSize={pointSize * 4}
                  className={styles.pointLabel}
                >
                  {position + 1}
                </text>
              </g>
            ))}
          {/* Drag handles of the selected source, at each shape centre. */}
          {showSources &&
            onReorder &&
            target &&
            !hidden.has(target.id) &&
            target.shapes.map((shape, position) => (
              <circle
                key={`handle-${position}`}
                data-shape-handle={position}
                className={`${styles.handle} ${shapeClass(position)}`}
                cx={shape.center[0]}
                cy={shape.center[1]}
                r={pointSize * 3}
                onPointerDown={(event) => {
                  if (busy) return
                  event.stopPropagation()
                  setDrag({
                    from: position,
                    x: shape.center[0],
                    y: shape.center[1],
                  })
                }}
              >
                <title>
                  {t('variation.compatibility.dragShape', {
                    index: position + 1,
                    source: target.name,
                  })}
                </title>
              </circle>
            ))}
          {drag && target?.shapes[drag.from] && (
            <line
              data-drag-line=""
              className={styles.dragLine}
              x1={target.shapes[drag.from]!.center[0]}
              y1={target.shapes[drag.from]!.center[1]}
              x2={drag.x}
              y2={drag.y}
            />
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
            <span className={styles.sourceName}>{source.name}</span>
            <small>
              {t('variation.compatibility.summary', {
                contours: source.shapes.filter((shape) => !shape.component)
                  .length,
                points: source.shapes.reduce(
                  (count, shape) => count + shape.points.length,
                  0,
                ),
              })}
            </small>
            {reference?.id === source.id && (
              <small>{t('variation.compatibility.reference')}</small>
            )}
          </label>
        ))}
      </fieldset>
      {onReorder && shapeCount > 0 && (
        <fieldset className={styles.legend} disabled={busy || !showSources}>
          <legend>{t('variation.compatibility.shapeOrder')}</legend>
          <label className={styles.sourceRow}>
            {t('variation.compatibility.reorderSource')}
            <select
              value={selectedSource}
              onChange={(event) =>
                setSelectedSource(Number(event.target.value))
              }
            >
              {sources.map((source) => (
                <option key={source.id} value={source.sourceIndex}>
                  {source.name}
                </option>
              ))}
            </select>
          </label>
          <p className={styles.guidance}>
            {t('variation.compatibility.reorderHint')}
          </p>
          {(target?.shapes ?? []).map((shape, position) => (
            <div key={`order-${position}`} className={styles.sourceRow}>
              <span
                className={`${styles.swatch} ${shapeClass(position)}`}
                aria-hidden="true"
              />
              <span className={styles.sourceName}>
                {shape.component
                  ? t('variation.compatibility.componentLabel', {
                      index: position + 1,
                      base: shape.component,
                    })
                  : t('variation.compatibility.shapeLabel', {
                      index: position + 1,
                    })}
              </span>
              <small
                data-shape-status={status[position]?.status ?? 'compatible'}
              >
                {t(
                  `variation.compatibility.status.${status[position]?.status ?? 'compatible'}`,
                )}
              </small>
              <Button
                disabled={position === 0}
                onClick={() => reorder(nudgeOrder(shapeCount, position, -1))}
              >
                {t('variation.compatibility.moveEarlier', {
                  index: position + 1,
                })}
              </Button>
              <Button
                disabled={position === shapeCount - 1}
                onClick={() => reorder(nudgeOrder(shapeCount, position, 1))}
              >
                {t('variation.compatibility.moveLater', {
                  index: position + 1,
                })}
              </Button>
            </div>
          ))}
        </fieldset>
      )}
      {!markersAvailable && markerCount > MAX_MARKERS && (
        <p className={styles.guidance}>
          {t('variation.compatibility.pointLimit')}
        </p>
      )}
      {sources.length >= 16 && (
        <p className={styles.guidance}>
          {t('variation.compatibility.sourceLimit')}
        </p>
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
