import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { call, type DocId, type RepertoireKind } from '../../api'
import { reportError, useUiStore } from '../../stores/uiStore'

import styles from './languageFilters.module.css'

/** The most missing characters one click preloads into the Add Glyphs recipe. */
export const MAX_GENERATED_RECIPE = 4_096

export interface GenerateMissingProps {
  doc: DocId
  /** Which grouping the repertoire belongs to. */
  kind: RepertoireKind
  /** Repertoire name, `Category/Subcategory` for a subcategory. */
  value: string
  /** Row label, used in the button's accessible name. */
  name: string
  /** How many characters of the repertoire are missing. */
  missing: number
}

/**
 * Fills out a character set: one click opens Add Glyphs with the repertoire's
 * missing characters already in the recipe.
 *
 * Add Glyphs is the review step on purpose — the person sees exactly what would
 * be created, and can build composites from the glyph database there. A
 * repertoire with more than {@link MAX_GENERATED_RECIPE} missing characters
 * preloads the first of them and says so, so a Unicode-wide category never
 * silently turns into an unreviewable batch.
 */
export function GenerateMissing({
  doc,
  kind,
  value,
  name,
  missing,
}: GenerateMissingProps) {
  const { t } = useTranslation()
  const setDialog = useUiStore((state) => state.setDialog)
  const [busy, setBusy] = useState(false)
  const label = t('sidebar.generateMissing', { name, count: missing })

  return (
    <button
      type="button"
      className={styles.generate}
      disabled={missing === 0 || busy}
      title={label}
      aria-label={label}
      onClick={() => {
        setBusy(true)
        void call('inventory.missing', {
          doc,
          kind,
          value,
          limit: MAX_GENERATED_RECIPE,
        })
          .then((result) => {
            if (result.scalars.length === 0) return
            if (result.total > result.scalars.length) {
              useUiStore.getState().pushToast({
                kind: 'info',
                message: t('sidebar.generateTruncated', {
                  count: result.scalars.length,
                  total: result.total,
                }),
              })
            }
            setDialog({ kind: 'addGlyphs', doc, recipe: result.recipe })
          })
          .catch(reportError)
          .finally(() => setBusy(false))
      }}
    >
      +
    </button>
  )
}
