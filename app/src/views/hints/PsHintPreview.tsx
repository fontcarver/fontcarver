import { useTranslation } from 'react-i18next'
import type { PsHints, StemHint } from '../../api'
import styles from './hints.module.css'

/** Filled stem bands and numbered edges make direction visible without color alone. */
export function PsHintPreview({
  path,
  hints,
  upem,
  flexPaths = [],
}: {
  path: string
  hints: PsHints
  upem: number
  flexPaths?: string[]
}) {
  const { t } = useTranslation()
  return (
    <svg
      className={styles.preview}
      viewBox={`${-upem * 0.1} ${-upem} ${upem * 1.2} ${upem * 1.3}`}
      role="img"
      aria-label={t('hints.preview')}
    >
      <g transform="scale(1,-1)">
        <path d={path} className={styles.outline} />
        {flexPaths.map((flex, i) =>
          flex ? <path key={`f${i}`} d={flex} className={styles.flex} /> : null,
        )}
        {hints.h_stems.map((stem, i) => (
          <g key={`h${i}`}>
            <rect
              x={0}
              y={edge(stem)}
              width={upem}
              height={Math.max(0, stem.width)}
              className={styles.band}
            />
            <path
              d={`M 0 ${edge(stem)} H ${upem}`}
              className={styles.horizontal}
            />
          </g>
        ))}
        {hints.v_stems.map((stem, i) => (
          <g key={`v${i}`}>
            <rect
              x={edge(stem)}
              y={-upem * 0.2}
              width={Math.max(0, stem.width)}
              height={upem * 1.2}
              className={styles.band}
            />
            <path
              d={`M ${edge(stem)} ${-upem * 0.2} V ${upem}`}
              className={styles.vertical}
            />
          </g>
        ))}
      </g>
      {hints.h_stems.map((stem, i) => (
        <text
          key={`h${i}`}
          x={-upem * 0.08}
          y={-edge(stem)}
          fontSize={upem * 0.04}
        >
          {i + 1}
        </text>
      ))}
      {hints.v_stems.map((stem, i) => (
        <text
          key={`v${i}`}
          x={edge(stem)}
          y={upem * 0.27}
          fontSize={upem * 0.04}
        >
          {hints.h_stems.length + i + 1}
        </text>
      ))}
    </svg>
  )
}

function edge(stem: StemHint): number {
  return stem.width === -21 ? stem.start + stem.width : stem.start
}
