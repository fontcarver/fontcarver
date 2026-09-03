import styles from './controls.module.css'

export interface SelectOption<T extends string> {
  value: T
  label: string
}

export interface SelectProps<T extends string> {
  label: string
  value: T
  options: readonly SelectOption<T>[]
  disabled?: boolean
  onChange: (value: T) => void
}

/** A native select styled to match the shell. */
export function Select<T extends string>({
  label,
  value,
  options,
  disabled,
  onChange,
}: SelectProps<T>) {
  return (
    <select
      aria-label={label}
      className={styles.select}
      disabled={disabled}
      value={value}
      onChange={(event) => onChange(event.target.value as T)}
    >
      {options.map((option) => (
        <option key={option.value} value={option.value}>
          {option.label}
        </option>
      ))}
    </select>
  )
}
