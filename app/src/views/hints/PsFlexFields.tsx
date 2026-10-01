import { useTranslation } from 'react-i18next'
import type { PsHints } from '../../api'
import { Button } from '../../components/Button'
import { NumberField } from '../../components/NumberField'
import styles from './hints.module.css'

/**
 * Flex hints and dot sections, both bound to stored outline identities.
 *
 * A flex hint names the on-curve point two consecutive curve segments leave, so
 * it follows that point through ordinary editing. A dot section names one closed
 * contour. Both are validated by the kernel when the draft is applied; an
 * unresolvable binding leaves the document untouched and keeps the draft.
 */
export function PsFlexFields({
  hints,
  contourCount,
  flexPaths,
  onChange,
}: {
  hints: PsHints
  contourCount: number
  flexPaths: string[]
  onChange: (hints: PsHints) => void
}) {
  const { t } = useTranslation()
  const lastContour = Math.max(0, contourCount - 1)
  // Both lists are omitted on the wire when empty.
  const flexHints = hints.flex_hints ?? []
  const dotSections = hints.dot_sections ?? []
  return (
    <>
      <h3>{t('hints.flexTitle')}</h3>
      <p>{t('hints.flexHelp')}</p>
      {hints.flex && <p role="alert">{t('hints.flexUnresolved')}</p>}
      {flexHints.map((flex, index) => (
        <fieldset key={index}>
          <legend>
            {t('hints.flexNumber', { number: index + 1 })}
            {flexPaths[index] === '' ? ` — ${t('hints.flexStale')}` : ''}
          </legend>
          <div className={styles.row}>
            {(['contour', 'point', 'depth'] as const).map((key) => (
              <NumberField
                key={key}
                label={t(`hints.${key === 'depth' ? 'flexDepth' : key}`)}
                value={flex[key]}
                onCommit={(value) =>
                  onChange({
                    ...hints,
                    flex_hints: flexHints.map((item, i) =>
                      i === index
                        ? {
                            ...item,
                            [key]: clamp(
                              value,
                              key === 'contour' ? lastContour : 1000,
                            ),
                          }
                        : item,
                    ),
                  })
                }
              />
            ))}
            <Button
              onClick={() =>
                onChange({
                  ...hints,
                  flex_hints: flexHints.filter((_, i) => i !== index),
                })
              }
            >
              {t('hints.removeFlex', { number: index + 1 })}
            </Button>
          </div>
        </fieldset>
      ))}
      <Button
        disabled={contourCount === 0 || flexHints.length >= 64}
        onClick={() =>
          onChange({
            ...hints,
            flex_hints: [...flexHints, { contour: 0, point: 0, depth: 50 }],
          })
        }
      >
        {t('hints.addFlex')}
      </Button>
      <h3>{t('hints.dotSections')}</h3>
      <p>{t('hints.dotSectionHelp')}</p>
      {dotSections.map((contour, index) => (
        <div className={styles.row} key={index}>
          <NumberField
            label={t('hints.contour')}
            value={contour}
            onCommit={(value) =>
              onChange({
                ...hints,
                dot_sections: dotSections.map((item, i) =>
                  i === index ? clamp(value, lastContour) : item,
                ),
              })
            }
          />
          <Button
            onClick={() =>
              onChange({
                ...hints,
                dot_sections: dotSections.filter((_, i) => i !== index),
              })
            }
          >
            {t('hints.removeDotSection', { number: index + 1 })}
          </Button>
        </div>
      ))}
      <Button
        disabled={
          contourCount === 0 ||
          dotSections.length >= contourCount ||
          dotSections.length >= 64
        }
        onClick={() =>
          onChange({
            ...hints,
            dot_sections: [
              ...dotSections,
              nextContour(dotSections, contourCount),
            ],
          })
        }
      >
        {t('hints.addDotSection')}
      </Button>
    </>
  )
}

/** Keeps an index inside the stored range so an edit cannot invalidate itself. */
function clamp(value: number, highest: number): number {
  return Math.min(Math.max(0, Math.round(value)), highest)
}

/** The lowest contour index that is not already a dot section. */
function nextContour(used: number[], count: number): number {
  for (let index = 0; index < count; index += 1) {
    if (!used.includes(index)) return index
  }
  return 0
}
