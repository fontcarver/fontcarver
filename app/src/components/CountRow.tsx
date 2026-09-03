import styles from './controls.module.css'

export interface CountRowProps {
  label: string
  value: number
}

/** A label with a right-aligned, tabular count. */
export function CountRow({ label, value }: CountRowProps) {
  return (
    <div className={styles.countRow}>
      <span className={styles.countLabel}>{label}</span>
      <span className={styles.countValue}>{value.toLocaleString()}</span>
    </div>
  )
}
