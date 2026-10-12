import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { TtHintState } from '../../api'
import { Button } from '../../components/Button'
import styles from './hints.module.css'

const PAGE = 32

/** Page point labels with their coordinates so dense glyphs stay inspectable. */
export function TtPointPreview({ state }: { state: TtHintState }) {
  const { t } = useTranslation()
  const [page, setPage] = useState(0)
  const start = page * PAGE
  const points = state.points.slice(start, start + PAGE)
  const bounds = state.points.reduce(
    (box, point) => ({
      x0: Math.min(box.x0, point.position.x),
      x1: Math.max(box.x1, point.position.x),
      y0: Math.min(box.y0, point.position.y),
      y1: Math.max(box.y1, point.position.y),
    }),
    {
      x0: Math.min(0, ...state.horizontal_phantoms),
      x1: Math.max(state.units_per_em, ...state.horizontal_phantoms),
      y0: -state.units_per_em * 0.2,
      y1: state.units_per_em,
    },
  )
  const padding = Math.max(bounds.x1 - bounds.x0, bounds.y1 - bounds.y0) * 0.08
  const labelSize =
    Math.max(bounds.x1 - bounds.x0, bounds.y1 - bounds.y0) * 0.025
  return (
    <section>
      <h3>{t('tt.points')}</h3>
      <svg
        className={styles.preview}
        viewBox={`${bounds.x0 - padding} ${-bounds.y1 - padding} ${bounds.x1 - bounds.x0 + padding * 2} ${bounds.y1 - bounds.y0 + padding * 2}`}
        role="img"
        aria-label={t('tt.points')}
      >
        <path
          d={state.path}
          transform="scale(1,-1)"
          className={styles.outline}
        />
        {points.map((point, index) => (
          <g key={start + index}>
            <circle
              cx={point.position.x}
              cy={-point.position.y}
              r={labelSize * 0.2}
              className={point.on_curve ? styles.onPoint : styles.offPoint}
            />
            <text
              x={point.position.x + labelSize * 0.3}
              y={-point.position.y - labelSize * 0.3}
              fontSize={labelSize}
            >
              {start + index}
            </text>
          </g>
        ))}
      </svg>
      <p>{t('tt.pointHelp')}</p>
      <div className={styles.row}>
        <Button disabled={page === 0} onClick={() => setPage(page - 1)}>
          {t('tt.previousPoints')}
        </Button>
        <span>
          {t('tt.pointRange', {
            start: Math.min(start, state.points.length),
            end: Math.max(0, Math.min(start + PAGE, state.points.length) - 1),
            count: state.points.length,
          })}
        </span>
        <Button
          disabled={start + PAGE >= state.points.length}
          onClick={() => setPage(page + 1)}
        >
          {t('tt.nextPoints')}
        </Button>
      </div>
      <div className={styles.pointTable}>
        <table>
          <thead>
            <tr>
              <th>{t('tt.index')}</th>
              <th>{t('tt.x')}</th>
              <th>{t('tt.y')}</th>
              <th>{t('tt.kind')}</th>
            </tr>
          </thead>
          <tbody>
            {points.map((point, index) => (
              <tr key={start + index}>
                <td>{start + index}</td>
                <td>{point.position.x}</td>
                <td>{point.position.y}</td>
                <td>{t(point.on_curve ? 'tt.onCurve' : 'tt.offCurve')}</td>
              </tr>
            ))}
            {Array.from({ length: 4 }, (_, i) => (
              <tr key={`phantom${i}`}>
                <td>{state.phantom_start + i}</td>
                <td>{i < 2 ? state.horizontal_phantoms[i] : 0}</td>
                <td>
                  {i < 2
                    ? 0
                    : (state.vertical_phantoms?.[i - 2] ?? t('tt.inferred'))}
                </td>
                <td>{t(`tt.phantom${i}`)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <p>{t('tt.phantomHelp')}</p>
      {state.component_metrics && <p>{t('tt.componentMetrics')}</p>}
    </section>
  )
}
