import type { ReactNode } from 'react'

import { useUiStore } from '../../../stores/uiStore'

import styles from './panels.module.css'

export interface PanelSectionProps {
  /** Stable id under which the open/closed state is remembered. */
  id: string
  title: string
  /** Count badge shown in the header; omitted where a count means nothing. */
  count?: number
  /** Whether the section starts open the first time it is ever shown. */
  defaultOpen?: boolean
  children: ReactNode
}

/**
 * One section of the Edit tab's inspector accordion (design spec §7).
 *
 * The open/closed state is remembered per section in the UI store, which
 * persists it the same way the panel sizes are persisted, so a workspace comes
 * back the way it was left rather than fully expanded every launch.
 */
export function PanelSection({
  id,
  title,
  count,
  defaultOpen = true,
  children,
}: PanelSectionProps) {
  const open = useUiStore((state) => state.sections[id] ?? defaultOpen)
  const toggle = useUiStore((state) => state.toggleSection)

  return (
    <section className={styles.section}>
      <button
        type="button"
        className={styles.header}
        aria-expanded={open}
        onClick={() => toggle(id, defaultOpen)}
      >
        <span className={styles.chevron} aria-hidden="true">
          {open ? '▾' : '▸'}
        </span>
        <span className={styles.headerTitle}>{title}</span>
        {count === undefined ? null : (
          <span className={styles.badge}>{count}</span>
        )}
      </button>
      {open ? <div className={styles.body}>{children}</div> : null}
    </section>
  )
}
