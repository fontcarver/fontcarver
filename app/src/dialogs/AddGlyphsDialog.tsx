import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { DocId, GlyphCell } from '../api'
import { Button } from '../components/Button'
import { Dialog } from '../components/Dialog'
import { useSessionStore } from '../stores/sessionStore'
import { reportError, useUiStore } from '../stores/uiStore'

import {
  CHARSETS,
  PREVIEW_DEBOUNCE_MS,
  applyRecipe,
  countRecipe,
  groupSkips,
  previewRecipe,
  type RecipeOutcome,
} from './addGlyphs'
import styles from './dialogs.module.css'
import { NameList } from './NameList'

const EMPTY: RecipeOutcome = { added: [], skipped: [], composed: [] }

/** How many skipped tokens a reason's tooltip names before it stops. */
const SKIP_SAMPLE = 20

/** The last dry run that came back, together with the recipe it answered. */
interface Preview {
  readonly recipe: string
  /** Whether the dry run built compositions, so a toggle invalidates it. */
  readonly compose: boolean
  readonly result: RecipeOutcome
}

/**
 * What one charset row shows once its dry run has come back: how many glyphs it
 * would add, and how many of its items were skipped for each reason. A row
 * whose dry run failed carries `null` instead, so it stops saying "counting…".
 */
interface CharsetCounts {
  readonly added: number
  readonly skipped: ReturnType<typeof groupSkips>
}

export interface AddGlyphsDialogProps {
  doc: DocId
  /** Recipe selected by a repertoire or saved glyph list. */
  recipe?: string
}

/**
 * Add Glyphs: a recipe field with syntax help, a character-set picker showing
 * how much of each repertoire is still missing, and a virtualised preview of
 * the names that would be created. Every count comes from a `glyphs.add` dry
 * run, so the dialog and the kernel can never disagree about a recipe.
 */
export function AddGlyphsDialog({
  doc,
  recipe: initialRecipe = '',
}: AddGlyphsDialogProps) {
  const { t } = useTranslation()
  const setDialog = useUiStore((state) => state.setDialog)
  const [recipe, setRecipe] = useState(initialRecipe)
  const [compose, setCompose] = useState(true)
  const [latest, setLatest] = useState<Preview | null>(null)
  const [busy, setBusy] = useState(false)
  const [counts, setCounts] = useState<Record<string, CharsetCounts | null>>({})

  const trimmed = recipe.trim()
  // Derived rather than stored: the preview belongs to exactly one recipe and
  // one composition setting, so "is a dry run still outstanding?" is a
  // comparison, not a second state.
  const fresh = latest?.recipe === trimmed && latest.compose === compose
  const preview = fresh ? latest.result : EMPTY
  const pending = trimmed.length > 0 && !fresh

  useEffect(() => {
    if (!trimmed) return
    let live = true
    const timer = setTimeout(() => {
      void previewRecipe(doc, trimmed, compose)
        .then((result) => {
          if (live) setLatest({ recipe: trimmed, compose, result })
        })
        .catch((error: unknown) => {
          if (!live) return
          setLatest({ recipe: trimmed, compose, result: EMPTY })
          reportError(error)
        })
    }, PREVIEW_DEBOUNCE_MS)
    return () => {
      live = false
      clearTimeout(timer)
    }
  }, [compose, doc, trimmed])

  useEffect(() => {
    let live = true
    void (async () => {
      // Sequential on purpose: a legacy CJK repertoire resolves thousands of
      // names, and the small Latin rows should not wait behind it.
      for (const charset of CHARSETS) {
        try {
          const result = await countRecipe(doc, charset.recipe)
          if (!live) return
          setCounts((current) => ({
            ...current,
            [charset.id]: {
              added: result.added,
              skipped: groupSkips(result.skipped),
            },
          }))
        } catch {
          // A repertoire the kernel cannot expand shows no counts rather than
          // counting for ever.
          if (!live) return
          setCounts((current) => ({ ...current, [charset.id]: null }))
        }
      }
    })()
    return () => {
      live = false
    }
  }, [doc])

  const close = () => setDialog(null)
  const cells: readonly GlyphCell[] = preview.added
  const composedCount = preview.composed.filter(
    (entry) => entry.components.length > 0,
  ).length
  const blocked = preview.composed.filter(
    (entry) => entry.unavailable.length > 0,
  )
  const blockedCount = blocked.length
  const blockedNames = [
    ...new Set(blocked.flatMap((entry) => entry.unavailable)),
  ]
    .slice(0, SKIP_SAMPLE)
    .join(' ')
  const canAdd = !busy && !pending && cells.length > 0
  const skipGroups = groupSkips(preview.skipped)

  const add = () => {
    if (!canAdd) return
    setBusy(true)
    void applyRecipe(doc, trimmed, compose)
      .then(async (result) => {
        await useSessionStore.getState().refreshSummary(doc)
        useUiStore.getState().pushToast({
          kind: 'info',
          message: t('toast.glyphsAdded', { count: result.added.length }),
        })
        close()
      })
      .catch((error: unknown) => {
        setBusy(false)
        reportError(error)
      })
  }

  return (
    <Dialog
      title={t('dialog.addGlyphs.title')}
      size="md"
      onCancel={close}
      onConfirm={add}
      footer={
        <>
          <Button onClick={close}>{t('common.cancel')}</Button>
          <Button variant="primary" disabled={!canAdd} onClick={add}>
            {t('dialog.addGlyphs.add', { count: cells.length })}
          </Button>
        </>
      }
    >
      <div className={styles.form}>
        <div className={styles.section}>
          <label className={styles.rowStack}>
            <span className={styles.sectionTitle}>
              {t('dialog.addGlyphs.recipe')}
            </span>
            <textarea
              autoFocus
              className={styles.recipe}
              value={recipe}
              onChange={(event) => setRecipe(event.target.value)}
            />
          </label>
          <p className={styles.hint}>{t('dialog.addGlyphs.recipeHint')}</p>
          <label className={styles.checkRow}>
            <input
              type="checkbox"
              checked={compose}
              onChange={(event) => setCompose(event.currentTarget.checked)}
            />
            <span>{t('dialog.addGlyphs.compose')}</span>
          </label>
          <p className={styles.hint}>{t('dialog.addGlyphs.composeHint')}</p>
        </div>

        <div className={styles.section}>
          <h3 className={styles.sectionTitle}>
            {t('dialog.addGlyphs.charsets')}
          </h3>
          <div className={styles.list}>
            {CHARSETS.map((charset) => {
              const count = counts[charset.id]
              return (
                <button
                  key={charset.id}
                  type="button"
                  className={styles.listRow}
                  onClick={() =>
                    setRecipe((current) =>
                      current.trim()
                        ? `${current.trim()} ${charset.recipe}`
                        : charset.recipe,
                    )
                  }
                >
                  <span>{t(`dialog.addGlyphs.charset.${charset.id}`)}</span>
                  <span className={styles.listCounts}>
                    {count === undefined ? (
                      <span>{t('dialog.addGlyphs.counting')}</span>
                    ) : count === null ? (
                      <span title={t('dialog.addGlyphs.countFailed')}>—</span>
                    ) : (
                      <>
                        <span>
                          {t('dialog.addGlyphs.new', { count: count.added })}
                        </span>
                        {count.skipped.map((group) => (
                          <span key={group.reason} className={styles.badge}>
                            {t(`dialog.addGlyphs.skip.${group.reason}`, {
                              count: group.items.length,
                            })}
                          </span>
                        ))}
                      </>
                    )}
                  </span>
                </button>
              )
            })}
          </div>
        </div>

        <div className={styles.section}>
          <h3 className={styles.sectionTitle}>
            {t('dialog.addGlyphs.preview')}
          </h3>
          <div className={styles.counts}>
            <span>{t('dialog.addGlyphs.new', { count: cells.length })}</span>
            {composedCount > 0 && (
              <span>
                {t('dialog.addGlyphs.composed', { count: composedCount })}
              </span>
            )}
            {blockedCount > 0 && (
              <span title={blockedNames}>
                {t('dialog.addGlyphs.composeBlocked', { count: blockedCount })}
              </span>
            )}
            {skipGroups.map((group) => (
              <span
                key={group.reason}
                // The first few tokens explain the count without a second list.
                title={group.items.slice(0, SKIP_SAMPLE).join(' ')}
              >
                {t(`dialog.addGlyphs.skip.${group.reason}`, {
                  count: group.items.length,
                })}
              </span>
            ))}
          </div>
          <NameList
            cells={cells}
            label={t('dialog.addGlyphs.preview')}
            empty={t('dialog.addGlyphs.empty')}
          />
        </div>
      </div>
    </Dialog>
  )
}
