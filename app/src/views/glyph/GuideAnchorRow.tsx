import { useTranslation } from 'react-i18next'

import type { Guide } from '../../api'

import {
  GUIDE_ANCHORS,
  reanchorGuide,
  resolvedGuideX,
  type InkEdges,
} from './guideAnchor'
import styles from './infoBox.module.css'

export interface GuideAnchorRowProps {
  guide: Guide
  edges: InkEdges
  /** Receives the anchor *and* the rebased offset, as one write. */
  onChange: (next: Pick<Guide, 'anchor' | 'x'>) => void
}

/**
 * The guide's LSB/RSB anchoring (design spec §6.1).
 *
 * Switching the anchor is one command, and it rebases the stored offset so the
 * line does not jump: what changes is what the guide follows from then on. The
 * resolved position is shown beside the selector, because a stored offset of
 * −30 units is only meaningful next to the absolute x it lands on.
 */
export function GuideAnchorRow({
  guide,
  edges,
  onChange,
}: GuideAnchorRowProps) {
  const { t } = useTranslation()
  const resolved = resolvedGuideX(guide, edges)
  return (
    <>
      <div className={styles.row}>
        <span className={styles.rowLabel}>{t('infoBox.anchor')}</span>
        <select
          className={styles.textInput}
          aria-label={t('infoBox.anchor')}
          title={t('infoBox.anchorHint')}
          value={guide.anchor}
          onChange={(event) => {
            const anchor = GUIDE_ANCHORS.find(
              (candidate) => candidate === event.currentTarget.value,
            )
            if (!anchor) return
            const next = reanchorGuide(guide, anchor, edges)
            onChange({ anchor: next.anchor, x: next.x })
          }}
        >
          {GUIDE_ANCHORS.map((anchor) => (
            <option key={anchor} value={anchor}>
              {t(`infoBox.anchorPoint.${anchor}`)}
            </option>
          ))}
        </select>
      </div>
      {guide.anchor !== 'Origin' && (
        <div className={styles.row}>
          <span className={styles.rowLabel}>{t('infoBox.x')}</span>
          <span className={styles.rowValue}>
            {Math.round(resolved * 100) / 100}
          </span>
        </div>
      )}
    </>
  )
}
