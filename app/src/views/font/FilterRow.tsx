import styles from './fontView.module.css'

export interface FilterRowProps {
  label: string
  count: number
  /** Optional repertoire size; count is the number of members present. */
  total?: number
  active: boolean
  /** Optional colour swatch shown before the label. */
  swatch?: string
  onToggle: () => void
}

/** One clickable sidebar filter with its glyph count. */
export function FilterRow({
  label,
  count,
  total,
  active,
  swatch,
  onToggle,
}: FilterRowProps) {
  return (
    <button
      type="button"
      className={styles.filterRow}
      aria-pressed={active}
      onClick={onToggle}
    >
      <span className={styles.filterLabel}>
        {swatch ? (
          <span
            aria-hidden="true"
            className={styles.swatch}
            style={{ background: swatch }}
          />
        ) : null}
        <span className={styles.filterName}>{label}</span>
      </span>
      <span className={styles.filterCount}>
        {count.toLocaleString()}
        {total === undefined ? '' : ` / ${total.toLocaleString()}`}
      </span>
    </button>
  )
}
