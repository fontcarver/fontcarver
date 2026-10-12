import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { MathKern, MathKerns, MathValue } from '../../api'
import { Button } from '../../components/Button'
import { MathValueField } from './MathValueField'
import styles from './math.module.css'

const CORNERS = [
  'top_right',
  'top_left',
  'bottom_right',
  'bottom_left',
] as const
const zero = (): MathValue => ({ value: 0, device: null })

/** Four optional corner kerning functions of one glyph. */
export function MathKernForm({
  kern,
  onChange,
}: {
  kern: MathKerns
  onChange: (kern: MathKerns) => void
}) {
  const { t } = useTranslation()
  return (
    <details className={styles.group}>
      <summary>{t('math.kerning')}</summary>
      <p>{t('math.kernHelp')}</p>
      {CORNERS.map((corner) => (
        <CornerKern
          key={corner}
          label={t(`math.corner.${corner}`)}
          value={kern[corner]}
          onChange={(value) => onChange({ ...kern, [corner]: value })}
        />
      ))}
    </details>
  )
}

function CornerKern({
  label,
  value,
  onChange,
}: {
  label: string
  value: MathKern | null
  onChange: (value: MathKern | null) => void
}) {
  const { t } = useTranslation()
  // Remount rows after a removal so unparsed text never shifts rows.
  const [epoch, setEpoch] = useState(0)
  return (
    <fieldset className={styles.card}>
      <legend>
        <label>
          <input
            type="checkbox"
            checked={value !== null}
            onChange={(event) =>
              onChange(
                event.target.checked ? { heights: [], values: [zero()] } : null,
              )
            }
          />
          {label}
        </label>
      </legend>
      {value &&
        value.values.map((kernValue, index) => {
          const height = value.heights[index]
          return (
            <div key={`${epoch}:${index}`} className={styles.kernRow}>
              <MathValueField
                label={t('math.kernValue', { index: index + 1 })}
                value={kernValue}
                onChange={(next) =>
                  onChange({
                    ...value,
                    values: value.values.map((v, i) =>
                      i === index ? next : v,
                    ),
                  })
                }
              />
              {height ? (
                <MathValueField
                  label={t('math.kernHeight', { index: index + 1 })}
                  value={height}
                  onChange={(next) =>
                    onChange({
                      ...value,
                      heights: value.heights.map((h, i) =>
                        i === index ? next : h,
                      ),
                    })
                  }
                />
              ) : (
                <span className={styles.muted}>{t('math.kernAbove')}</span>
              )}
              {height && (
                <Button
                  onClick={() => {
                    setEpoch((e) => e + 1)
                    onChange({
                      heights: value.heights.filter((_, i) => i !== index),
                      values: value.values.filter((_, i) => i !== index),
                    })
                  }}
                >
                  {t('math.removeRow')}
                </Button>
              )}
            </div>
          )
        })}
      {value && (
        <Button
          onClick={() => {
            const last = value.heights.at(-1)?.value ?? 0
            onChange({
              heights: [...value.heights, { value: last + 100, device: null }],
              values: [...value.values, zero()],
            })
          }}
        >
          {t('math.addKernRow')}
        </Button>
      )}
    </fieldset>
  )
}
