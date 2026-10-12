import { useTranslation } from 'react-i18next'

import type { GlyphStatus } from './glyphStatuses'
import styles from './glyphStatusBadges.module.css'

/** Compact, noninteractive indicators; the parent exposes their full meaning. */
export function GlyphStatusBadges({
  statuses,
  size,
}: {
  statuses: readonly GlyphStatus[]
  size: number
}) {
  const { t } = useTranslation()
  if (statuses.length === 0) return null

  // Leave room for the component indicator and keep badges clear of the label.
  const capacity = Math.max(1, Math.floor((size - 18) / 14))
  const visible = statuses.slice(
    0,
    statuses.length > capacity ? capacity - 1 : capacity,
  )
  const remaining = statuses.slice(visible.length)

  return (
    <span className={styles.badges} aria-hidden="true">
      {visible.map(({ id, labelKey, Icon, tone }) => (
        <span
          key={id}
          className={styles.badge}
          data-tone={tone}
          data-glyph-status={id}
          title={t(labelKey)}
        >
          <Icon size={11} strokeWidth={2} />
        </span>
      ))}
      {remaining.length > 0 && (
        <span
          className={styles.badge}
          data-tone={
            remaining.some((status) => status.tone === 'warning')
              ? 'warning'
              : 'neutral'
          }
          data-status-overflow
          title={remaining.map(({ labelKey }) => t(labelKey)).join(', ')}
        >
          +{remaining.length}
        </span>
      )}
    </span>
  )
}
