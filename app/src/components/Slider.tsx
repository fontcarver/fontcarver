import styles from './controls.module.css'

export interface SliderProps {
  label: string
  value: number
  min: number
  max: number
  step?: number
  disabled?: boolean
  /** Text shown right of the track, e.g. the current value with its unit. */
  valueLabel?: string
  onChange: (value: number) => void
}

/** A range input with an optional numeric read-out. */
export function Slider({
  label,
  value,
  min,
  max,
  step = 1,
  disabled,
  valueLabel,
  onChange,
}: SliderProps) {
  return (
    <span className={styles.sliderRow}>
      <input
        type="range"
        aria-label={label}
        className={styles.slider}
        disabled={disabled}
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(event) => onChange(Number(event.target.value))}
      />
      {valueLabel === undefined ? null : (
        <span className={styles.sliderValue}>{valueLabel}</span>
      )}
    </span>
  )
}
