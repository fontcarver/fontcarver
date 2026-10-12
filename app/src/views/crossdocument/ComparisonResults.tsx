import { useTranslation } from 'react-i18next'
import type { FontCompareResult, MergeChoice } from '../../api'
import { Button } from '../../components/Button'
import { ComparisonSections } from './ComparisonSections'
import styles from './crossDocument.module.css'

/** Per-glyph resolution: keep the target, take the reference, or leave it alone. */
export type GlyphResolution = MergeChoice | 'skip'

/** Paginated glyph differences with an explicit per-conflict resolution. */
export function ComparisonResults({
  result,
  busy,
  choices,
  onChoice,
  onInspect,
  onPage,
}: {
  result: FontCompareResult | null
  busy: boolean
  choices: ReadonlyMap<string, MergeChoice>
  onChoice: (name: string, choice: GlyphResolution) => void
  onInspect: (name: string) => void
  onPage: (offset: number) => void
}) {
  const { t } = useTranslation()
  if (!result) return null
  const failures = Object.entries(result.compatibility)
    .filter(([, matches]) => !matches)
    .map(([key]) => t(`crossDocument.compatibility.${key}`))
  return (
    <section
      aria-label={t('crossDocument.diffResults')}
      className={styles.form}
    >
      <p role="status">
        {t('crossDocument.diffCounts', {
          equal: result.counts.equal,
          outlines: result.counts.outlines,
          metrics: result.counts.metrics,
          missing: result.counts.missing_target + result.counts.missing_source,
          unicode: result.counts.unicodes,
          components: result.counts.components,
        })}
      </p>
      <p className={styles.help}>
        {t('crossDocument.compatibilityTitle')}:{' '}
        {failures.length === 0
          ? t('crossDocument.compatible')
          : failures.join(', ')}
      </p>
      <div className={styles.results}>
        <table className={styles.table}>
          <thead>
            <tr>
              <th>{t('crossDocument.resolution')}</th>
              <th>{t('crossDocument.glyph')}</th>
              <th>{t('crossDocument.differences')}</th>
            </tr>
          </thead>
          <tbody>
            {result.glyphs.map((glyph) => (
              <tr
                key={glyph.name}
                className={
                  choices.has(glyph.name) ? styles.selected : undefined
                }
              >
                <td>
                  <select
                    disabled={busy || glyph.source === null}
                    aria-label={t('crossDocument.resolveGlyph', {
                      name: glyph.name,
                    })}
                    value={choices.get(glyph.name) ?? 'skip'}
                    onChange={(event) =>
                      onChoice(
                        glyph.name,
                        event.target.value as GlyphResolution,
                      )
                    }
                  >
                    {(['skip', 'target', 'source'] as const).map((value) => (
                      <option key={value} value={value}>
                        {t(`crossDocument.resolutions.${value}`)}
                      </option>
                    ))}
                  </select>
                </td>
                <td>
                  <Button
                    disabled={busy}
                    onClick={() => {
                      onInspect(glyph.name)
                    }}
                  >
                    {glyph.name}
                  </Button>
                </td>
                <td>
                  {[
                    glyph.status.startsWith('missing')
                      ? t(`crossDocument.status.${glyph.status}`)
                      : null,
                    glyph.outline_changed ? t('crossDocument.outlines') : null,
                    glyph.metrics_changed ? t('crossDocument.metrics') : null,
                    glyph.unicode_changed ? t('crossDocument.unicode') : null,
                    glyph.components_changed
                      ? t('crossDocument.components')
                      : null,
                    !glyph.compatible &&
                    glyph.source !== null &&
                    glyph.target !== null
                      ? t('crossDocument.topology')
                      : null,
                    glyph.status === 'equal' ? t('crossDocument.equal') : null,
                    ...glyph.issues,
                  ]
                    .filter(Boolean)
                    .join(', ')}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className={styles.controls}>
        <Button
          disabled={busy || result.offset === 0}
          onClick={() => {
            onPage(Math.max(0, result.offset - result.limit))
          }}
        >
          {t('crossDocument.previous')}
        </Button>
        <span>
          {t('crossDocument.page', {
            start: result.total === 0 ? 0 : result.offset + 1,
            end: Math.min(result.offset + result.glyphs.length, result.total),
            total: result.total,
          })}
        </span>
        <Button
          disabled={busy || result.offset + result.limit >= result.total}
          onClick={() => {
            onPage(result.offset + result.limit)
          }}
        >
          {t('crossDocument.next')}
        </Button>
      </div>
      <ComparisonSections result={result} />
    </section>
  )
}
