import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import styles from './panels.module.css'
import { metricsKeyError, normalizeMetricsKey } from './metricsKeyCommands'

export interface MetricsKeyFieldProps {
  /** Already-translated field label, used for the accessible name. */
  label: string
  /** Formula currently stored for this metric, with its scope prefix. */
  value: string
  /** Example formula shown as the placeholder. */
  placeholder: string
  /** Called with the normalised formula only when it is syntactically valid. */
  onCommit: (value: string) => void
  disabled?: boolean
  /** A reconciliation restores a rejected draft even if the value is unchanged. */
  revision?: unknown
}

/**
 * One metrics formula field with inline validation and a syntax tooltip.
 *
 * The formula grammar is small but not guessable, so the field carries the
 * whole syntax in its tooltip and describes it to screen readers. Validation
 * mirrors the kernel grammar and happens before the command: a typo is reported
 * in place and the document is never touched, while the draft stays on screen
 * so it can be corrected rather than retyped. Enter and blur commit, Escape
 * reverts to the stored formula, and a bare `H` becomes `=H`.
 */
export function MetricsKeyField({
  label,
  value,
  placeholder,
  onCommit,
  disabled = false,
  revision,
}: MetricsKeyFieldProps) {
  const { t } = useTranslation()
  const [text, setText] = useState(value)
  const [shown, setShown] = useState(value)
  const [shownRevision, setShownRevision] = useState(revision)
  const [rejected, setRejected] = useState<string | null>(null)
  const cancelBlur = useRef(false)
  if (value !== shown || revision !== shownRevision) {
    setShown(value)
    setShownRevision(revision)
    setText(value)
    setRejected(null)
  }

  const syntax = t('panel.metrics.keySyntax')
  const live = metricsKeyError(text)
  const problem = live ?? rejected
  const commit = (raw: string): void => {
    if (disabled) return
    const next = normalizeMetricsKey(raw)
    const error = metricsKeyError(next)
    if (error) {
      setRejected(error)
      return
    }
    setRejected(null)
    if (next === value) {
      setText(next)
      return
    }
    onCommit(next)
  }

  return (
    <label className={styles.row}>
      <span className={styles.rowLabel}>{label}</span>
      <input
        className={styles.textInput}
        type="text"
        spellCheck={false}
        disabled={disabled}
        value={text}
        aria-label={label}
        aria-invalid={problem ? true : undefined}
        aria-describedby="metrics-key-syntax"
        placeholder={placeholder}
        title={
          problem
            ? `${t(`panel.metrics.keyError.${problem}`)}\n${syntax}`
            : syntax
        }
        onChange={(event) => {
          setRejected(null)
          setText(event.currentTarget.value)
        }}
        onFocus={(event) => event.currentTarget.select()}
        onBlur={(event) => {
          if (cancelBlur.current) {
            cancelBlur.current = false
            return
          }
          commit(event.currentTarget.value)
        }}
        onKeyDown={(event) => {
          if (event.key === 'Enter') {
            event.preventDefault()
            commit(event.currentTarget.value)
            return
          }
          if (event.key !== 'Escape') return
          cancelBlur.current = true
          setText(value)
          setRejected(null)
          event.currentTarget.blur()
        }}
      />
      {problem && (
        <span className={styles.warning} role="alert">
          {t(`panel.metrics.keyError.${problem}`)}
        </span>
      )}
    </label>
  )
}
