import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

/** A list keeps incomplete input visible and exposes validity to its parent form. */
export function NumberListField({
  label,
  value,
  onChange,
}: {
  label: string
  value: number[]
  onChange: (value: number[]) => void
}) {
  const { t } = useTranslation()
  const serialized = value.join(', ')
  const [draft, setDraft] = useState({
    source: serialized,
    text: serialized,
    valid: true,
  })
  const input = useRef<HTMLInputElement>(null)
  const current = draft.source === serialized
  useEffect(() => {
    input.current?.setCustomValidity(
      current && !draft.valid ? t('hints.invalidList') : '',
    )
  }, [current, draft.valid, t])
  return (
    <label>
      {label}
      <input
        ref={input}
        value={current ? draft.text : serialized}
        onChange={(event) => {
          const raw = event.target.value
          const values =
            raw.trim() === '' ? [] : raw.split(/[,\s]+/).map(Number)
          const valid = values.every(Number.isFinite) && !/[,\s]$/.test(raw)
          setDraft({
            source: valid ? values.join(', ') : serialized,
            text: raw,
            valid,
          })
          event.target.setCustomValidity(valid ? '' : t('hints.invalidList'))
          if (valid) onChange(values)
        }}
      />
    </label>
  )
}
