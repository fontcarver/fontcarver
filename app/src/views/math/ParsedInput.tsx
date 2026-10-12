import { useEffect, useRef, useState } from 'react'

/**
 * Text input that keeps incomplete input visible and reports only parsed values.
 *
 * `parse` returns `undefined` for invalid text; the input then carries a
 * custom validity message so the surrounding form refuses to apply.
 */
export function ParsedInput<T>({
  label,
  value,
  format,
  parse,
  invalid,
  onChange,
  placeholder,
  inputMode,
  list,
  required,
  onText,
}: {
  label: string
  value: T
  format: (value: T) => string
  parse: (text: string) => T | undefined
  invalid: string
  onChange: (value: T) => void
  placeholder?: string
  inputMode?: 'numeric' | 'decimal' | 'text'
  list?: string
  required?: boolean
  /** Receives raw text on every keystroke, for suggestions. */
  onText?: (text: string) => void
}) {
  const serialized = format(value)
  const [draft, setDraft] = useState({
    source: serialized,
    text: serialized,
    valid: true,
  })
  const input = useRef<HTMLInputElement>(null)
  const current = draft.source === serialized
  useEffect(() => {
    input.current?.setCustomValidity(current && !draft.valid ? invalid : '')
  }, [current, draft.valid, invalid])
  return (
    <input
      ref={input}
      aria-label={label}
      data-validation-message={invalid}
      placeholder={placeholder}
      inputMode={inputMode}
      list={list}
      required={required}
      spellCheck={false}
      value={current ? draft.text : serialized}
      onChange={(event) => {
        const text = event.target.value
        onText?.(text)
        const parsed = parse(text)
        const valid = parsed !== undefined
        setDraft({
          source: valid ? format(parsed) : serialized,
          text,
          valid,
        })
        event.target.setCustomValidity(valid ? '' : invalid)
        if (valid) onChange(parsed)
      }}
    />
  )
}

/** Parse a finite number within inclusive bounds, rejecting empty text. */
export function parseNumber(
  min: number,
  max: number,
): (text: string) => number | undefined {
  return (text) => {
    if (text.trim() === '') return undefined
    const value = Number(text)
    return Number.isFinite(value) &&
      Math.round(value) >= min &&
      Math.round(value) <= max
      ? value
      : undefined
  }
}

/** Signed 16-bit font-unit values, checked after the binary rounding. */
export const parseSigned = parseNumber(-32768, 32767)
/** Unsigned 16-bit font-unit values, checked after the binary rounding. */
export const parseUnsigned = parseNumber(0, 65535)
