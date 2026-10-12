import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import type { TextGap } from './textGaps'
import { MAX_VISIBLE_RUN_GLYPHS } from './textRunGeometry'
import styles from './TextGapPicker.module.css'

/** Keyboard-accessible counterpart of the canvas's shaped gap handles. */
export function TextGapPicker({
  gaps,
  selected,
  onSelect,
  pending,
  visible,
}: {
  gaps: readonly TextGap[]
  selected: number | null
  onSelect: (index: number) => void
  pending: boolean
  visible: readonly number[]
}) {
  const { t } = useTranslation()
  const shown = useMemo(() => {
    const indexes = visible.length
      ? visible
      : gaps.slice(0, MAX_VISIBLE_RUN_GLYPHS).map((gap) => gap.index)
    return [...new Set([...(selected === null ? [] : [selected]), ...indexes])]
      .sort((a, b) => a - b)
      .flatMap((index) => (gaps[index] ? [gaps[index]!] : []))
  }, [gaps, selected, visible])
  return (
    <div className={styles.row}>
      <label className={styles.label}>
        {t('kerningCanvas.label')}
        <select
          aria-label={t('kerningCanvas.label')}
          value={selected ?? ''}
          disabled={pending || !gaps.some((gap) => gap.reason === null)}
          onChange={(event) => {
            if (event.currentTarget.value !== '')
              onSelect(Number(event.currentTarget.value))
          }}
        >
          <option value="">{t('kerningCanvas.choose')}</option>
          {shown.map((gap) => (
            <option
              key={gap.index}
              value={gap.index}
              disabled={gap.reason !== null}
            >
              {t('kerningCanvas.pair', {
                index: gap.index + 1,
                left: gap.left ?? t('kerningCanvas.unknownGlyph'),
                right: gap.right ?? t('kerningCanvas.unknownGlyph'),
              })}
              {gap.reason ? ` — ${t(`kerningCanvas.${gap.reason}`)}` : ''}
            </option>
          ))}
        </select>
      </label>
      <span className={styles.hint}>{t('kerningCanvas.help')}</span>
    </div>
  )
}
