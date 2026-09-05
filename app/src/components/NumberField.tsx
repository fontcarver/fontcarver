import { useEffect, useRef, useState } from 'react'

import { evaluateField, formatUnits } from '../editor/fieldValue'

import styles from './controls.module.css'

export interface NumberFieldProps {
  label: string
  value: number
  /** Called with the committed value; only fires when the value changed. */
  onCommit: (value: number) => void
  disabled?: boolean
  /** Increment for ↑↓; Shift multiplies it by ten. */
  step?: number
  /** Tooltip shown when the entry cannot be parsed. */
  invalidHint?: string
}

/**
 * A numeric field with the design spec's §7 behaviour: select-all on focus,
 * expressions (`+10`, `*2`) against the current value, ↑↓ stepping (⇧ ×10),
 * Enter or blur commits one undo step, Esc reverts, and an invalid entry
 * shakes for 120 ms instead of writing anything.
 */
export function NumberField({
  label,
  value,
  onCommit,
  disabled = false,
  step = 1,
  invalidHint,
}: NumberFieldProps) {
  const [text, setText] = useState(() => formatUnits(value))
  const [invalid, setInvalid] = useState(false)
  const editing = useRef(false)

  // Follow the model unless the user is mid-edit, which would fight them.
  useEffect(() => {
    if (!editing.current) setText(formatUnits(value))
  }, [value])

  useEffect(() => {
    if (!invalid) return
    const timer = setTimeout(() => setInvalid(false), 120)
    return () => clearTimeout(timer)
  }, [invalid])

  const commit = (raw: string): boolean => {
    const next = evaluateField(raw, value)
    if (next === null) {
      setInvalid(true)
      setText(formatUnits(value))
      return false
    }
    setText(formatUnits(next))
    if (next !== value) onCommit(next)
    return true
  }

  return (
    <label
      className={`${styles.field} ${invalid ? styles.fieldInvalid : ''}`}
      title={invalid ? invalidHint : undefined}
    >
      <span className={styles.fieldLabel}>{label}</span>
      <input
        className={styles.numberInput}
        type="text"
        inputMode="numeric"
        value={text}
        disabled={disabled}
        aria-label={label}
        aria-invalid={invalid || undefined}
        onFocus={(event) => {
          editing.current = true
          event.currentTarget.select()
        }}
        onChange={(event) => setText(event.currentTarget.value)}
        onBlur={(event) => {
          editing.current = false
          commit(event.currentTarget.value)
        }}
        onKeyDown={(event) => {
          if (event.key === 'Enter') {
            event.preventDefault()
            if (commit(event.currentTarget.value)) editing.current = false
            return
          }
          if (event.key === 'Escape') {
            event.preventDefault()
            editing.current = false
            setText(formatUnits(value))
            event.currentTarget.blur()
            return
          }
          if (event.key !== 'ArrowUp' && event.key !== 'ArrowDown') return
          event.preventDefault()
          const amount = step * (event.shiftKey ? 10 : 1)
          const base = evaluateField(event.currentTarget.value, value) ?? value
          const next = event.key === 'ArrowUp' ? base + amount : base - amount
          setText(formatUnits(next))
          if (next !== value) onCommit(next)
        }}
      />
    </label>
  )
}
