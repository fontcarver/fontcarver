import type { ReactNode } from 'react'

import styles from './controls.module.css'

export interface PanelProps {
  title: string
  /** Controls rendered at the right of the panel header. */
  actions?: ReactNode
  className?: string
  children: ReactNode
}

/** A titled section of the sidebar or inspector. */
export function Panel({ title, actions, className, children }: PanelProps) {
  return (
    <section
      className={className ? `${styles.panel} ${className}` : styles.panel}
    >
      <header className={styles.panelHeader}>
        <span>{title}</span>
        {actions}
      </header>
      <div className={styles.panelBody}>{children}</div>
    </section>
  )
}
