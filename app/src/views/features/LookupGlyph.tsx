import { useTranslation } from 'react-i18next'

import { call, type DocId, type GlyphId, type MasterId } from '../../api'

import styles from './LookupInspector.module.css'
import { useLayoutQuery } from './useLayoutQuery'

/** One persistent glyph reference from a bounded layout rule. */
export interface LookupGlyphProps {
  doc: DocId
  master: MasterId
  generation: number
  glyph: { name: string; gid: GlyphId | null }
}

/** Draws the font's own outline, including glyphs with no Unicode mapping. */
export function LookupGlyph({
  doc,
  master,
  generation,
  glyph,
}: LookupGlyphProps) {
  const { t } = useTranslation()
  const identity = JSON.stringify([
    doc,
    master,
    generation,
    glyph.gid,
    glyph.name,
  ])
  const answer = useLayoutQuery(identity, async () => {
    if (glyph.gid === null) return null
    return call('glyphs.cell', { doc, master, gid: glyph.gid })
  })
  const cell = answer?.value
  const bounds = cell?.name === glyph.name ? cell.bbox : null
  const width = bounds ? Math.max(1, bounds[2] - bounds[0]) : 1
  const height = bounds ? Math.max(1, bounds[3] - bounds[1]) : 1
  const padding = Math.max(width, height) * 0.1
  return (
    <span className={styles.glyph}>
      {bounds && cell?.path ? (
        <svg
          role="img"
          aria-label={t('layoutInspector.glyphOutline', { name: glyph.name })}
          className={styles.glyphOutline}
          viewBox={`${bounds[0] - padding} ${-bounds[3] - padding} ${width + padding * 2} ${height + padding * 2}`}
          preserveAspectRatio="xMidYMid meet"
        >
          <path d={cell.path} transform="scale(1 -1)" />
        </svg>
      ) : (
        <span
          className={styles.glyphPlaceholder}
          title={t(
            answer?.failed
              ? 'layoutInspector.outlineFailed'
              : 'layoutInspector.noOutline',
          )}
          aria-hidden="true"
        />
      )}
      <code>{glyph.name}</code>
    </span>
  )
}
