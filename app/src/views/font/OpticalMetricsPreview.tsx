import { useTranslation } from 'react-i18next'
import type { OpticalExample, OpticalOperation } from '../../api'
import styles from './opticalMetricsPreview.module.css'

/** Review captured metrics and spacing outlines at the same before/after scale. */
export function OpticalMetricsPreview({
  examples,
  operation,
}: {
  examples: OpticalExample[]
  operation: OpticalOperation
}) {
  const { t, i18n } = useTranslation()
  const numbers = new Intl.NumberFormat(i18n.resolvedLanguage, {
    maximumFractionDigits: 3,
  })
  const spacing = operation === 'spacing'

  if (examples.length === 0) return null

  return (
    <div
      className={styles.examples}
      role="region"
      aria-label={t('opticalSpacing.examplesLabel')}
      tabIndex={0}
    >
      {examples.slice(0, 16).map((example, index) => {
        const glyphs = example.right
          ? t('opticalSpacing.pairLabel', {
              left: example.glyph,
              right: example.right,
            })
          : example.glyph
        const shift = example.after[0] - example.before[0]
        const [x0, y0, x1, y1] = example.bounds
        const [afterX0, afterY0, afterX1, afterY1] = example.after_bounds ?? [
          x0 + shift,
          y0,
          x1 + shift,
          y1,
        ]
        const left = Math.min(0, x0, afterX0)
        const right = Math.max(x1, afterX1, example.before[2], example.after[2])
        const bottom = Math.min(0, y0, afterY0)
        const top = Math.max(0, y1, afterY1)
        const padding = Math.max(right - left, top - bottom, 1) * 0.08
        const viewBox = [
          left - padding,
          -top - padding,
          right - left + padding * 2,
          top - bottom + padding * 2,
        ].join(' ')

        return (
          <table className={styles.metrics} key={index}>
            <caption>
              {t('opticalSpacing.exampleCaption', {
                glyphs,
                master: example.master,
              })}
            </caption>
            <thead>
              <tr>
                <th scope="col">{t('opticalSpacing.state')}</th>
                {spacing ? (
                  <>
                    <th scope="col">{t('opticalSpacing.lsb')}</th>
                    <th scope="col">{t('opticalSpacing.rsb')}</th>
                    <th scope="col">{t('opticalSpacing.width')}</th>
                    <th scope="col">{t('opticalSpacing.outline')}</th>
                  </>
                ) : (
                  <th scope="col">{t('opticalSpacing.effectiveKerning')}</th>
                )}
              </tr>
            </thead>
            <tbody>
              {(['before', 'after'] as const).map((state) => (
                <tr key={state}>
                  <th scope="row">{t(`opticalSpacing.${state}`)}</th>
                  {example[state].slice(0, spacing ? 3 : 1).map((value, i) => (
                    <td key={i} title={String(value)}>
                      {numbers.format(value)}
                    </td>
                  ))}
                  {spacing && (
                    <td className={styles.outlineCell}>
                      <svg
                        className={styles.outline}
                        viewBox={viewBox}
                        aria-hidden="true"
                        focusable="false"
                      >
                        <g transform="scale(1 -1)">
                          <line
                            className={styles.guide}
                            x1={left}
                            x2={right}
                            y1={0}
                            y2={0}
                          />
                          {[0, example[state][2]].map((advance, i) => (
                            <line
                              key={i}
                              className={styles.guide}
                              x1={advance}
                              x2={advance}
                              y1={bottom}
                              y2={top}
                            />
                          ))}
                          <path
                            d={
                              state === 'after'
                                ? (example.after_outline ?? example.outline)
                                : example.outline
                            }
                            transform={`translate(${state === 'after' && !example.after_outline ? shift : 0} 0)`}
                          />
                        </g>
                      </svg>
                    </td>
                  )}
                </tr>
              ))}
            </tbody>
          </table>
        )
      })}
    </div>
  )
}
