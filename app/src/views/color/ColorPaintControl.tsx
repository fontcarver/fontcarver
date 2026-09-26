import { useTranslation } from 'react-i18next'
import { ParsedInput } from '../math/ParsedInput'
import styles from './color.module.css'

/** Large palettes use an index field instead of thousands of repeated options. */
export function ColorPaintControl({
  index,
  count,
  number,
  validate,
  onChange,
}: {
  index: number | null
  count: number
  number: number
  validate: () => boolean
  onChange: (index: number | null) => void
}) {
  const { t } = useTranslation()
  const label = t('color.paint', { number })
  if (count <= 256)
    return (
      <select
        aria-label={label}
        value={index ?? 'foreground'}
        onChange={(e) =>
          onChange(
            e.target.value === 'foreground' ? null : Number(e.target.value),
          )
        }
      >
        <option value="foreground">{t('color.foreground')}</option>
        {Array.from({ length: count }, (_, i) => (
          <option key={i} value={i}>
            {t('color.entry', { number: i })}
          </option>
        ))}
      </select>
    )
  return (
    <div className={styles.paint}>
      <label>
        <input
          type="checkbox"
          aria-label={`${label}: ${t('color.foreground')}`}
          checked={index === null}
          onChange={(e) => {
            if (validate()) onChange(e.target.checked ? null : 0)
          }}
        />
        {t('color.foreground')}
      </label>
      {index !== null && (
        <ParsedInput
          label={label}
          value={index}
          format={String}
          parse={(text) =>
            /^\d+$/.test(text) && Number(text) < count
              ? Number(text)
              : undefined
          }
          invalid={t('color.invalidEntryIndex', { max: count - 1 })}
          required
          inputMode="numeric"
          onChange={onChange}
        />
      )}
    </div>
  )
}
