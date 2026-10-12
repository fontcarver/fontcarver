import { useRef, useState } from 'react'

import styles from './panels.module.css'

export interface TextRowProps {
  label: string
  value: string
  /** Called with the trimmed text on Enter or blur, only when it changed. */
  onCommit: (value: string) => void
  placeholder?: string
  /** Render a multi-line box rather than a single-line field. */
  multiline?: boolean
  /** Reject an empty commit, restoring the model value instead. */
  required?: boolean
  /** Disable editing when the current workspace locks this field. */
  disabled?: boolean
  /** A reconciliation also restores a rejected draft when the value is unchanged. */
  revision?: unknown
}

/**
 * A text field that follows the model.
 *
 * An uncontrolled input would keep whatever was typed into it, so a value the
 * kernel changed — an undo, an edit from the canvas — would leave a stale
 * string on screen. This one notices during render that the model moved and
 * adopts it, which is React's documented way to follow a prop without an
 * effect. Enter and blur commit (design spec §7); Escape reverts.
 */
export function TextRow({
  label,
  value,
  onCommit,
  placeholder,
  multiline = false,
  required = false,
  disabled = false,
  revision,
}: TextRowProps) {
  const [text, setText] = useState(value)
  const [shown, setShown] = useState(value)
  const [shownRevision, setShownRevision] = useState(revision)
  const cancelBlur = useRef(false)
  if (value !== shown || revision !== shownRevision) {
    setShown(value)
    setShownRevision(revision)
    setText(value)
  }

  const commit = (raw: string): void => {
    if (disabled) return
    const next = raw.trim()
    if (required && next === '') {
      setText(value)
      return
    }
    if (next === value) return
    onCommit(next)
  }

  const shared = {
    disabled,
    value: text,
    'aria-label': label,
    placeholder,
    onChange: (event: { currentTarget: { value: string } }) =>
      setText(event.currentTarget.value),
    onFocus: (event: { currentTarget: { select: () => void } }) =>
      event.currentTarget.select(),
    onBlur: (event: { currentTarget: { value: string } }) => {
      if (cancelBlur.current) {
        cancelBlur.current = false
        return
      }
      commit(event.currentTarget.value)
    },
  }

  return (
    <label className={styles.row}>
      <span className={styles.rowLabel}>{label}</span>
      {multiline ? (
        <textarea
          {...shared}
          className={styles.textArea}
          onKeyDown={(event) => {
            if (event.key !== 'Escape') return
            cancelBlur.current = true
            event.currentTarget.value = value
            setText(value)
            event.currentTarget.blur()
          }}
        />
      ) : (
        <input
          {...shared}
          className={styles.textInput}
          onKeyDown={(event) => {
            if (event.key === 'Enter') {
              event.preventDefault()
              event.currentTarget.blur()
              return
            }
            if (event.key !== 'Escape') return
            cancelBlur.current = true
            event.currentTarget.value = value
            setText(value)
            event.currentTarget.blur()
          }}
        />
      )}
    </label>
  )
}
