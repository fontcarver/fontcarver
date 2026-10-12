import styles from './fontView.module.css'

export interface FilterRowProps {
  label: string
  /**
   * What the badge shows on the left of the slash.
   *
   * Omitted for a row that has nothing to count, such as a badge filter.
   */
  count?: number
  /** Optional repertoire size; count is the number of members present. */
  total?: number
  active: boolean
  /** Optional colour swatch shown before the label. */
  swatch?: string
  /** Extra explanation for the row, shown as its tooltip. */
  title?: string
  onToggle: () => void
}

/** One clickable sidebar filter with its glyph count. */
export function FilterRow({
  label,
  count,
  total,
  active,
  swatch,
  title,
  onToggle,
}: FilterRowProps) {
  return (
    <button
      type="button"
      className={styles.filterRow}
      aria-pressed={active}
      title={title}
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
        {count === undefined ? '' : count.toLocaleString()}
        {total === undefined ? '' : ` / ${total.toLocaleString()}`}
      </span>
    </button>
  )
}
