import { useTranslation } from 'react-i18next'
import type { MasterMetrics } from '../../api'
import { NumberField } from '../../components/NumberField'
import { Button } from '../../components/Button'
import styles from './designSpace.module.css'

const FIELDS = [
  ['ascender', 'ascender'],
  ['descender', 'descender'],
  ['x_height', 'xHeight'],
  ['cap_height', 'capHeight'],
  ['italic_angle', 'italicAngle'],
  ['hhea_ascender', 'hheaAscender'],
  ['hhea_descender', 'hheaDescender'],
  ['hhea_line_gap', 'hheaLineGap'],
  ['typo_ascender', 'typoAscender'],
  ['typo_descender', 'typoDescender'],
  ['typo_line_gap', 'typoLineGap'],
  ['win_ascent', 'winAscent'],
  ['win_descent', 'winDescent'],
  ['vhea_ascender', 'vheaAscender'],
  ['vhea_descender', 'vheaDescender'],
  ['vhea_line_gap', 'vheaLineGap'],
  ['default_vertical_origin', 'verticalOrigin'],
  ['default_vertical_advance', 'verticalAdvance'],
] as const
const REQUIRED = new Set<keyof MasterMetrics>([
  'ascender',
  'descender',
  'x_height',
  'cap_height',
  'italic_angle',
])

/** Edits the draft master metrics; the parent commits them atomically. */
export function MasterMetricsEditor({
  metrics,
  onChange,
}: {
  metrics: MasterMetrics
  onChange: (metrics: MasterMetrics) => void
}) {
  const { t } = useTranslation()
  return (
    <details>
      <summary>{t('variation.masterMetrics')}</summary>
      <div className={styles.form}>
        {FIELDS.map(([field, label]) => (
          <div key={field} className={styles.row}>
            {!REQUIRED.has(field) && (
              <label>
                <input
                  type="checkbox"
                  checked={metrics[field] !== null}
                  aria-label={t('variation.overrideMetric', {
                    metric: t(`fontInfo.metric.${label}`),
                  })}
                  onChange={(e) =>
                    onChange({
                      ...metrics,
                      [field]: e.target.checked
                        ? field === 'default_vertical_advance'
                          ? 1000
                          : field === 'default_vertical_origin'
                            ? metrics.ascender
                            : 0
                        : null,
                    })
                  }
                />
                {t('variation.override')}
              </label>
            )}
            <NumberField
              label={t(`fontInfo.metric.${label}`)}
              value={metrics[field] ?? 0}
              disabled={metrics[field] === null}
              onCommit={(value) => onChange({ ...metrics, [field]: value })}
            />
          </div>
        ))}
        <Button
          onClick={() =>
            onChange({
              ...metrics,
              default_vertical_advance: null,
              default_vertical_origin: null,
              vhea_ascender: null,
              vhea_descender: null,
              vhea_line_gap: null,
            })
          }
        >
          {t('variation.resetVertical')}
        </Button>
      </div>
    </details>
  )
}
