import type { ReactNode } from 'react'

import styles from './controls.module.css'

export interface FieldProps {
  label: string
  /** Read-only rows render as a plain row instead of a form label. */
  readOnly?: boolean
  children: ReactNode
}

/** A labelled row in the inspector: label on the left, value or control right. */
export function Field({ label, readOnly = false, children }: FieldProps) {
  const content = (
    <>
      <span className={styles.fieldLabel}>{label}</span>
      <span className={styles.fieldValue}>{children}</span>
    </>
  )
  return readOnly ? (
    <div className={styles.field}>{content}</div>
  ) : (
    <label className={styles.field}>{content}</label>
  )
}
