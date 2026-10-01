import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  call,
  type TtAutohintInfo,
  type TtAutohintOptions,
  type TtAutohinterStatus,
} from '../../api'
import styles from './hints.module.css'

/** Defaults that pass kernel validation and reproduce the tool's own defaults. */
export const DEFAULT_TT_AUTOHINT: TtAutohintOptions = {
  x_height_snapping_exceptions: '',
  default_script: '',
  fallback_script: '',
  strong_stem_width: '',
  adjust_subglyphs: false,
  composites: false,
  dehint: false,
  symbol: false,
  ttfa_table: false,
  windows_compatibility: false,
  info: 'None',
}

/** Whether every option still matches the tool's own defaults. */
export function isDefaultTtAutohint(options: TtAutohintOptions): boolean {
  const canonical = (value: TtAutohintOptions) =>
    JSON.stringify(
      Object.entries({ ...DEFAULT_TT_AUTOHINT, ...value })
        .filter(([, item]) => item !== undefined)
        .sort(([a], [b]) => a.localeCompare(b)),
    )
  return canonical(options) === canonical(DEFAULT_TT_AUTOHINT)
}

/**
 * Each numeric option and the range the kernel accepts for it.
 *
 * A ppem is at least 2; a hinting limit and an x-height increase may also be
 * zero, which switches that behaviour off. The kernel checks the same bounds and
 * the relations between them, and reports what it refuses.
 */
const NUMBERS = [
  ['hinting_range_min', 2],
  ['hinting_range_max', 2],
  ['hinting_limit', 0],
  ['fallback_stem_width', 1],
  ['increase_x_height', 0],
] as const

const FLAGS = [
  'adjust_subglyphs',
  'composites',
  'dehint',
  'symbol',
  'ttfa_table',
  'windows_compatibility',
] as const

const TEXTS = [
  'default_script',
  'fallback_script',
  'strong_stem_width',
  'x_height_snapping_exceptions',
] as const

const INFO: TtAutohintInfo[] = ['None', 'Short', 'Detailed']

/** Keeps a typed value inside the range the kernel accepts for that option. */
function clampOption(value: number, lowest: number): number {
  if (!Number.isFinite(value)) return lowest
  const rounded = Math.round(value)
  // Zero stays available where it means "switch this behaviour off".
  if (lowest === 0 && rounded <= 0) return 0
  return Math.min(10000, Math.max(lowest, rounded))
}

/**
 * The external TrueType hinter's availability and its documented options.
 *
 * The availability line is read before the export runs, so an absent executable
 * is disclosed in advance instead of only in the export report. Every value is
 * validated in the kernel before the process starts; an invalid value fails the
 * export request rather than being dropped.
 */
export function TtAutohintFields({
  options,
  busy,
  onChange,
}: {
  options: TtAutohintOptions
  busy: boolean
  onChange: (options: TtAutohintOptions) => void
}) {
  const { t } = useTranslation()
  const [status, setStatus] = useState<TtAutohinterStatus | null>(null)
  useEffect(() => {
    let current = true
    // The kernel owns the option rules, so it is asked whether this draft would
    // be accepted instead of repeating them here.
    void call('hints.tt_autohinter', { options })
      .then((result) => {
        if (current) setStatus(result)
      })
      .catch(() => {
        if (current) setStatus(null)
      })
    return () => {
      current = false
    }
  }, [options])
  return (
    <fieldset disabled={busy}>
      <legend>{t('hints.ttAutohintTitle')}</legend>
      <p role="status">
        {status === null
          ? t('hints.ttAutohintUnknown')
          : status.installed
            ? t('hints.ttAutohintFound', {
                version: status.version || status.path,
              })
            : t('hints.ttAutohintMissing')}
      </p>
      {status !== null && status.problem !== '' && (
        <p role="alert">
          {t('hints.ttAutohintInvalid')} {status.problem}
        </p>
      )}
      <div className={styles.row}>
        {NUMBERS.map(([key, lowest]) => (
          <label key={key}>
            {t(`hints.ttAutohint.${key}`)}
            <input
              type="number"
              min={lowest}
              max={10000}
              step={1}
              value={options[key] ?? ''}
              onChange={(event) =>
                onChange({
                  ...options,
                  // An empty field leaves the tool's own default in place.
                  [key]:
                    event.target.value === ''
                      ? undefined
                      : clampOption(Number(event.target.value), lowest),
                })
              }
            />
          </label>
        ))}
      </div>
      <p>{t('hints.ttAutohintNumbersHelp')}</p>
      <div className={styles.row}>
        {TEXTS.map((key) => (
          <label key={key}>
            {t(`hints.ttAutohint.${key}`)}
            <input
              type="text"
              value={options[key]}
              onChange={(event) =>
                onChange({ ...options, [key]: event.target.value })
              }
            />
          </label>
        ))}
      </div>
      <p>{t('hints.ttAutohintTextHelp')}</p>
      <div className={styles.row}>
        {FLAGS.map((key) => (
          <label key={key}>
            <input
              type="checkbox"
              checked={options[key]}
              onChange={(event) =>
                onChange({ ...options, [key]: event.target.checked })
              }
            />
            {t(`hints.ttAutohint.${key}`)}
          </label>
        ))}
      </div>
      <label>
        {t('hints.ttAutohint.info')}
        <select
          value={options.info}
          onChange={(event) =>
            onChange({ ...options, info: event.target.value as TtAutohintInfo })
          }
        >
          {INFO.map((value) => (
            <option key={value} value={value}>
              {t(`hints.ttAutohintInfo.${value}`)}
            </option>
          ))}
        </select>
      </label>
    </fieldset>
  )
}
