import { useTranslation } from 'react-i18next'
import type { DocSummary, FontCompareResult } from '../../api'
import { Button } from '../../components/Button'
import { documentLabel } from '../../stores/sessionStore'
import styles from './crossDocument.module.css'

/** Family compatibility and paginated glyph differences with explicit selection. */
export function ComparisonResults({
  family,
  result,
  busy,
  selected,
  onSelected,
  onInspect,
  onPage,
}: {
  family: { doc: DocSummary; result: FontCompareResult }[]
  result: FontCompareResult | null
  busy: boolean
  selected: ReadonlySet<string>
  onSelected: (names: Set<string>) => void
  onInspect: (name: string) => void
  onPage: (offset: number) => void
}) {
  const { t } = useTranslation()
  const compatibilityText = (comparison: FontCompareResult) => {
    const failures = Object.entries(comparison.compatibility)
      .filter(([, matches]) => !matches)
      .map(([key]) => t(`crossDocument.compatibility.${key}`))
    return failures.length === 0
      ? t('crossDocument.compatible')
      : failures.join(', ')
  }
  return (
    <>
      {family.length > 0 && (
        <section aria-label={t('crossDocument.familyResults')}>
          <p className={styles.help}>{t('crossDocument.familyHelp')}</p>
          <table className={styles.table}>
            <thead>
              <tr>
                <th>{t('crossDocument.source')}</th>
                <th>{t('crossDocument.compatibilityTitle')}</th>
              </tr>
            </thead>
            <tbody>
              {family.map((row) => (
                <tr key={row.doc.doc}>
                  <td>
                    {documentLabel(row.doc)} · {row.doc.style_name}
                  </td>
                  <td>{compatibilityText(row.result)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}
      {result && (
        <section
          aria-label={t('crossDocument.diffResults')}
          className={styles.form}
        >
          <p role="status">
            {t('crossDocument.diffCounts', {
              equal: result.counts.equal,
              outlines: result.counts.outlines,
              metrics: result.counts.metrics,
              missing:
                result.counts.missing_target + result.counts.missing_source,
              unicode: result.counts.unicodes,
            })}
          </p>
          <p className={styles.help}>
            {t('crossDocument.compatibilityTitle')}: {compatibilityText(result)}
          </p>
          <div className={styles.results}>
            <table className={styles.table}>
              <thead>
                <tr>
                  <th>{t('crossDocument.select')}</th>
                  <th>{t('crossDocument.glyph')}</th>
                  <th>{t('crossDocument.differences')}</th>
                </tr>
              </thead>
              <tbody>
                {result.glyphs.map((glyph) => (
                  <tr
                    key={glyph.name}
                    className={
                      selected.has(glyph.name) ? styles.selected : undefined
                    }
                  >
                    <td>
                      <input
                        type="checkbox"
                        disabled={busy || glyph.source === null}
                        aria-label={t('crossDocument.selectGlyph', {
                          name: glyph.name,
                        })}
                        checked={selected.has(glyph.name)}
                        onChange={(event) => {
                          const next = new Set(selected)
                          if (event.target.checked) next.add(glyph.name)
                          else next.delete(glyph.name)
                          onSelected(next)
                        }}
                      />
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
                        glyph.outline_changed
                          ? t('crossDocument.outlines')
                          : null,
                        glyph.metrics_changed
                          ? t('crossDocument.metrics')
                          : null,
                        glyph.unicode_changed
                          ? t('crossDocument.unicode')
                          : null,
                        !glyph.compatible &&
                        glyph.source !== null &&
                        glyph.target !== null
                          ? t('crossDocument.topology')
                          : null,
                        glyph.status === 'equal'
                          ? t('crossDocument.equal')
                          : null,
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
                end: Math.min(
                  result.offset + result.glyphs.length,
                  result.total,
                ),
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
        </section>
      )}
    </>
  )
}
