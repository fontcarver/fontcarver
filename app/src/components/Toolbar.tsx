import type { ReactNode } from 'react'

import styles from './controls.module.css'

export interface ToolbarProps {
  label: string
  children: ReactNode
}

/** A horizontal strip of controls above a view. */
export function Toolbar({ label, children }: ToolbarProps) {
  return (
    <div role="toolbar" aria-label={label} className={styles.toolbar}>
      {children}
    </div>
  )
}
