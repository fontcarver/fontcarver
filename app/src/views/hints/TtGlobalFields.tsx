import { useTranslation } from 'react-i18next'
import type { TtMasterData } from '../../api'
import styles from './hints.module.css'

const maxima = [
  'max_zones',
  'max_twilight_points',
  'max_storage',
  'max_function_defs',
  'max_instruction_defs',
  'max_stack_elements',
] as const

export function TtGlobalFields({
  value,
  cvt,
  onCvt,
  onChange,
}: {
  value: TtMasterData
  cvt: string
  onCvt: (text: string) => void
  onChange: (data: TtMasterData) => void
}) {
  const { t } = useTranslation()
  return (
    <fieldset>
      <legend>{t('tt.globals')}</legend>
      <p>{t('tt.globalsHelp')}</p>
      <label className={styles.sourceField}>
        {t('tt.cvt')}
        <textarea
          rows={3}
          spellCheck={false}
          value={cvt}
          onChange={(event) => {
            const raw = event.target.value
            const values = raw.trim()
              ? raw
                  .trim()
                  .split(/[,\s]+/)
                  .map(Number)
              : []
            const valid =
              values.length <= 65536 &&
              values.every(
                (value) =>
                  Number.isInteger(value) && value >= -32768 && value <= 32767,
              ) &&
              !/,$/.test(raw.trim())
            event.target.setCustomValidity(valid ? '' : t('tt.invalidCvt'))
            onCvt(raw)
            if (valid) onChange({ ...value, cvt: values })
          }}
        />
      </label>
      <div className={styles.fields}>
        {maxima.map((key) => (
          <label key={key}>
            {t(`tt.${key}`)}
            <input
              type="number"
              required
              step={1}
              min={key === 'max_zones' ? 1 : 0}
              max={
                key === 'max_zones'
                  ? 2
                  : key === 'max_instruction_defs'
                    ? 256
                    : 65535
              }
              value={value[key] ?? ''}
              onChange={(event) =>
                onChange({
                  ...value,
                  [key]:
                    event.target.value === ''
                      ? null
                      : event.target.valueAsNumber,
                })
              }
            />
          </label>
        ))}
      </div>
    </fieldset>
  )
}
