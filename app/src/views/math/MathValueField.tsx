import { useTranslation } from 'react-i18next'
import type { MathDevice, MathValue } from '../../api'
import { formatDevice, parseDevice } from './mathModel'
import { ParsedInput, parseSigned } from './ParsedInput'
import styles from './math.module.css'

/** A font-unit value with optional per-size pixel corrections. */
export function MathValueField({
  label,
  value,
  onChange,
}: {
  label: string
  value: MathValue
  onChange: (value: MathValue) => void
}) {
  const { t } = useTranslation()
  return (
    <div className={styles.value}>
      <span>{label}</span>
      <ParsedInput<number>
        label={label}
        value={value.value}
        format={String}
        parse={parseSigned}
        invalid={t('math.invalidSigned')}
        inputMode="decimal"
        onChange={(next) => onChange({ ...value, value: next })}
      />
      <ParsedInput<MathDevice | null>
        label={t('math.deviceFor', { name: label })}
        value={value.device}
        format={formatDevice}
        parse={parseDevice}
        invalid={t('math.invalidDevice')}
        placeholder={t('math.devicePlaceholder')}
        onChange={(device) => onChange({ ...value, device })}
      />
    </div>
  )
}

/** An optional value with a presence checkbox. */
export function OptionalMathValue({
  label,
  value,
  onChange,
}: {
  label: string
  value: MathValue | null
  onChange: (value: MathValue | null) => void
}) {
  const { t } = useTranslation()
  return (
    <div className={styles.optional}>
      <label>
        <input
          type="checkbox"
          checked={value !== null}
          onChange={(event) =>
            onChange(event.target.checked ? { value: 0, device: null } : null)
          }
        />
        {t('math.present', { name: label })}
      </label>
      {value && (
        <MathValueField label={label} value={value} onChange={onChange} />
      )}
    </div>
  )
}
