import { useTranslation } from 'react-i18next'
import type { MathConstants, MathTable } from '../../api'
import { CONSTANT_GROUPS, emptyConstants } from './mathModel'
import { MathValueField } from './MathValueField'
import { ParsedInput, parseUnsigned } from './ParsedInput'
import styles from './math.module.css'

const parseInteger = (text: string) => {
  const value = Number(text)
  return text.trim() !== '' &&
    Number.isInteger(value) &&
    value >= -32768 &&
    value <= 32767
    ? value
    : undefined
}

const INTEGER_CONSTANTS = [
  'script_percent_scale_down',
  'script_script_percent_scale_down',
  'radical_degree_bottom_raise_percent',
] as const
const HEIGHT_CONSTANTS = [
  'delimited_sub_formula_min_height',
  'display_operator_min_height',
] as const

/** Global constants in specification groups, plus the connector overlap. */
export function MathConstantsForm({
  table,
  onChange,
}: {
  table: MathTable
  onChange: (table: MathTable) => void
}) {
  const { t } = useTranslation()
  const constants = table.constants
  const set = (patch: Partial<MathConstants>) =>
    constants && onChange({ ...table, constants: { ...constants, ...patch } })
  return (
    <div className={styles.section}>
      <div className={styles.row}>
        <label>
          {t('math.minConnectorOverlap')}
          <ParsedInput<number>
            label={t('math.minConnectorOverlap')}
            value={table.min_connector_overlap}
            format={String}
            parse={parseUnsigned}
            invalid={t('math.invalidUnsigned')}
            inputMode="decimal"
            onChange={(min_connector_overlap) =>
              onChange({ ...table, min_connector_overlap })
            }
          />
        </label>
        <label>
          <input
            type="checkbox"
            checked={constants !== null}
            onChange={(event) =>
              onChange({
                ...table,
                constants: event.target.checked ? emptyConstants() : null,
              })
            }
          />
          {t('math.hasConstants')}
        </label>
      </div>
      <p>{t('math.overlapHelp')}</p>
      {constants && (
        <>
          <div className={styles.grid}>
            {INTEGER_CONSTANTS.map((name) => (
              <label key={name}>
                {t(`math.constant.${name}`)}
                <ParsedInput<number>
                  label={t(`math.constant.${name}`)}
                  value={constants[name]}
                  format={String}
                  parse={parseInteger}
                  invalid={t('math.invalidInteger')}
                  inputMode="numeric"
                  onChange={(value) => set({ [name]: value })}
                />
              </label>
            ))}
            {HEIGHT_CONSTANTS.map((name) => (
              <label key={name}>
                {t(`math.constant.${name}`)}
                <ParsedInput<number>
                  label={t(`math.constant.${name}`)}
                  value={constants[name]}
                  format={String}
                  parse={parseUnsigned}
                  invalid={t('math.invalidUnsigned')}
                  inputMode="decimal"
                  onChange={(value) => set({ [name]: value })}
                />
              </label>
            ))}
          </div>
          <p>{t('math.deviceHelp')}</p>
          {Object.entries(CONSTANT_GROUPS).map(([group, names]) => (
            <details
              key={group}
              className={styles.group}
              open={group === 'general'}
            >
              <summary>{t(`math.group.${group}`)}</summary>
              {names.map((name) => (
                <MathValueField
                  key={name}
                  label={t(`math.constant.${name}`)}
                  value={constants[name]}
                  onChange={(value) => set({ [name]: value })}
                />
              ))}
            </details>
          ))}
        </>
      )}
    </div>
  )
}
