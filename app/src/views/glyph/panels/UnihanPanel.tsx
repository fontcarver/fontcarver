import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { call, type UnihanInfo } from '../../../api'
import { PanelSection } from './PanelSection'
import styles from './panels.module.css'

const FIELDS = [
  ['radical_strokes', 'radicalStrokes'],
  ['total_strokes', 'totalStrokes'],
  ['mandarin', 'languages.mandarin'],
  ['cantonese', 'languages.cantonese'],
  ['japanese_on', 'japaneseOn'],
  ['japanese_kun', 'japaneseKun'],
  ['korean', 'languages.korean'],
  ['iicore', 'charsets.iicore'],
] as const

/** Original offline Unicode properties for every encoded ideograph in a glyph. */
export function UnihanPanel({ unicodes }: { unicodes: readonly number[] }) {
  const { t } = useTranslation()
  const key = unicodes.join(',')
  const [result, setResult] = useState<{
    key: string
    records: UnihanInfo[]
  } | null>(null)
  useEffect(() => {
    let cancelled = false
    const points = key ? key.split(',').map(Number) : []
    void Promise.all(
      points.map((unicode) => call('glyphs.unihan', { unicode })),
    )
      .then((records) => {
        if (!cancelled)
          setResult({
            key,
            records: records.filter(
              (record): record is UnihanInfo => record !== null,
            ),
          })
      })
      .catch(() => {
        if (!cancelled) setResult({ key, records: [] })
      })
    return () => {
      cancelled = true
    }
  }, [key])
  if (result?.key !== key || !result.records.length) return null
  return (
    <PanelSection id="unihan" title={t('cjk.metadata')}>
      {result.records.map((record) => (
        <div key={record.unicode}>
          <div className={styles.row}>
            <span className={styles.rowValue}>
              {`U+${record.unicode.toString(16).toUpperCase().padStart(4, '0')} ${String.fromCodePoint(record.unicode)}`}
            </span>
          </div>
          {FIELDS.filter(([field]) => record[field]).map(([field, label]) => (
            <div className={styles.row} key={field}>
              <span className={styles.rowLabel}>{t(`cjk.${label}`)}</span>
              <span className={styles.rowValue}>{record[field]}</span>
            </div>
          ))}
        </div>
      ))}
      <p className={styles.rowLabel}>{t('cjk.sourceHint')}</p>
    </PanelSection>
  )
}
