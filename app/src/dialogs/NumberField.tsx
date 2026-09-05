import { useState } from 'react'

import styles from './dialogs.module.css'

export interface NumberFieldProps {
  /** Accessible name of the field. */
  label: string
  /** Committed value; what the field shows whenever it is not being edited. */
  value: number
  min?: number
  max?: number
  step?: number
  /** Called once per committed edit, never per keystroke. */
  onCommit: (value: number) => void
}

/**
 * A numeric field that reports a value only once the user is done with it.
 *
 * Preferences are written straight through to the kernel, so a field that
 * reported every keystroke would persist the `0` of a momentarily empty box on
 * the way to the real number. This one keeps the draft locally and commits on
 * blur or Enter; Escape abandons it. Enter does not travel further, so it
 * commits the field instead of also confirming the dialog around it.
 */
export function NumberField({
  label,
  value,
  min,
  max,
  step,
  onCommit,
}: NumberFieldProps) {
  const [draft, setDraft] = useState<string | null>(null)
  const shown = draft ?? String(value)

  const commit = (): void => {
    setDraft(null)
    if (draft === null) return
    const parsed = Number(draft)
    if (draft.trim() === '' || !Number.isFinite(parsed)) return
    // Both bounds are optional and independent: falling back to `parsed` in
    // either place would hand the raw value back and undo the other clamp.
    const clamped = Math.min(
      max ?? Number.POSITIVE_INFINITY,
      Math.max(min ?? Number.NEGATIVE_INFINITY, parsed),
    )
    if (clamped !== value) onCommit(clamped)
  }

  return (
    <input
      type="number"
      aria-label={label}
      className={styles.number}
      min={min}
      max={max}
      step={step}
      value={shown}
      onChange={(event) => setDraft(event.target.value)}
      onBlur={commit}
      onKeyDown={(event) => {
        if (event.key === 'Enter') {
          event.stopPropagation()
          commit()
        } else if (event.key === 'Escape' && draft !== null) {
          event.stopPropagation()
          setDraft(null)
        }
      }}
    />
  )
}
