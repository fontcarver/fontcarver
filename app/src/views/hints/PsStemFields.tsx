import { useTranslation } from 'react-i18next'
import type { PsHints } from '../../api'
import { Button } from '../../components/Button'
import { NumberField } from '../../components/NumberField'
import styles from './hints.module.css'

export function PsStemFields({
  hints,
  onChange,
}: {
  hints: PsHints
  onChange: (hints: PsHints) => void
}) {
  const { t } = useTranslation()
  const count = hints.h_stems.length + hints.v_stems.length
  return (
    <>
      <p>{t('hints.stemHelp')}</p>
      {(['h_stems', 'v_stems'] as const).map((axis) => (
        <fieldset key={axis}>
          <legend>{t(`hints.${axis}`)}</legend>
          {hints[axis].map((stem, index) => (
            <div className={styles.row} key={index}>
              <span>
                {(axis === 'v_stems' ? hints.h_stems.length : 0) + index + 1}
              </span>
              {(['start', 'width'] as const).map((key) => (
                <NumberField
                  key={key}
                  label={t(`hints.${key}`)}
                  value={stem[key]}
                  onCommit={(value) =>
                    onChange({
                      ...hints,
                      [axis]: hints[axis].map((item, i) =>
                        i === index ? { ...item, [key]: value } : item,
                      ),
                    })
                  }
                />
              ))}
              <Button
                disabled={hints.hint_masks.length > 0}
                onClick={() =>
                  onChange({
                    ...hints,
                    [axis]: hints[axis].filter((_, i) => i !== index),
                  })
                }
              >
                {t('hints.remove')}
              </Button>
            </div>
          ))}
          <Button
            disabled={count >= 96 || hints.hint_masks.length > 0}
            onClick={() =>
              onChange({
                ...hints,
                [axis]: [...hints[axis], { start: 0, width: 80, ranges: [] }],
              })
            }
          >
            {t('hints.addStem')}
          </Button>
        </fieldset>
      ))}
      <h3>{t('hints.masks')}</h3>
      <p>{t('hints.maskHelp')}</p>
      {hints.hint_masks.map((mask, index) => (
        <fieldset key={index}>
          <div className={styles.row}>
            {(['contour', 'point'] as const).map((key) => (
              <NumberField
                key={key}
                label={t(`hints.${key}`)}
                value={mask[key]}
                onCommit={(value) =>
                  onChange({
                    ...hints,
                    hint_masks: hints.hint_masks.map((item, i) =>
                      i === index ? { ...item, [key]: value } : item,
                    ),
                  })
                }
              />
            ))}
            {(['counter', 'closing'] as const).map((key) => (
              <label key={key}>
                <input
                  type="checkbox"
                  checked={mask[key]}
                  onChange={(event) =>
                    onChange({
                      ...hints,
                      hint_masks: hints.hint_masks.map((item, i) =>
                        i === index
                          ? { ...item, [key]: event.target.checked }
                          : item,
                      ),
                    })
                  }
                />
                {t(`hints.${key}`)}
              </label>
            ))}
            <Button
              onClick={() =>
                onChange({
                  ...hints,
                  hint_masks: hints.hint_masks.filter((_, i) => i !== index),
                })
              }
            >
              {t('hints.remove')}
            </Button>
          </div>
          <div className={styles.row}>
            {Array.from({ length: count }, (_, bit) => (
              <label key={bit}>
                <input
                  type="checkbox"
                  checked={Boolean(
                    (mask.mask[Math.floor(bit / 8)] ?? 0) & (0x80 >> (bit % 8)),
                  )}
                  onChange={(event) => {
                    const bytes = [...mask.mask]
                    const byte = Math.floor(bit / 8),
                      flag = 0x80 >> (bit % 8)
                    bytes[byte] = event.target.checked
                      ? (bytes[byte] ?? 0) | flag
                      : (bytes[byte] ?? 0) & ~flag
                    onChange({
                      ...hints,
                      hint_masks: hints.hint_masks.map((item, i) =>
                        i === index ? { ...item, mask: bytes } : item,
                      ),
                    })
                  }}
                />
                {t('hints.stemNumber', { number: bit + 1 })}
              </label>
            ))}
          </div>
        </fieldset>
      ))}
      <Button
        disabled={count === 0}
        onClick={() =>
          onChange({
            ...hints,
            hint_masks: [
              ...hints.hint_masks,
              {
                contour: 0,
                point: 0,
                counter: false,
                closing: false,
                mask: Array.from({ length: Math.ceil(count / 8) }, () => 0),
              },
            ],
          })
        }
      >
        {t('hints.addMask')}
      </Button>
    </>
  )
}
