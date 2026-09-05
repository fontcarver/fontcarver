import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { AddGlyphsResult, DocId, GlyphCell } from '../api'
import { Button } from '../components/Button'
import { Dialog } from '../components/Dialog'
import { errorMessage, useSessionStore } from '../stores/sessionStore'
import { useUiStore } from '../stores/uiStore'

import {
  CHARSETS,
  PREVIEW_DEBOUNCE_MS,
  applyRecipe,
  previewRecipe,
} from './addGlyphs'
import styles from './dialogs.module.css'
import { NameList } from './NameList'

const EMPTY: AddGlyphsResult = { added: [], skipped: [] }

/** The last dry run that came back, together with the recipe it answered. */
interface Preview {
  readonly recipe: string
  readonly result: AddGlyphsResult
}

/** Counts one charset row shows once its dry run has come back. */
interface CharsetCounts {
  readonly added: number
  readonly present: number
}

export interface AddGlyphsDialogProps {
  doc: DocId
}

/**
 * Add Glyphs: a recipe field with syntax help, a character-set picker showing
 * how much of each repertoire is still missing, and a virtualised preview of
 * the names that would be created. Every count comes from a `glyphs.add` dry
 * run, so the dialog and the kernel can never disagree about a recipe.
 */
export function AddGlyphsDialog({ doc }: AddGlyphsDialogProps) {
  const { t } = useTranslation()
  const setDialog = useUiStore((state) => state.setDialog)
  const [recipe, setRecipe] = useState('')
  const [latest, setLatest] = useState<Preview | null>(null)
  const [busy, setBusy] = useState(false)
  const [counts, setCounts] = useState<Record<string, CharsetCounts>>({})

  const trimmed = recipe.trim()
  // Derived rather than stored: the preview belongs to exactly one recipe, so
  // "is a dry run still outstanding?" is a comparison, not a second state.
  const preview = latest?.recipe === trimmed ? latest.result : EMPTY
  const pending = trimmed.length > 0 && latest?.recipe !== trimmed

  useEffect(() => {
    if (!trimmed) return
    let live = true
    const timer = setTimeout(() => {
      void previewRecipe(doc, trimmed)
        .then((result) => {
          if (live) setLatest({ recipe: trimmed, result })
        })
        .catch((error: unknown) => {
          if (!live) return
          setLatest({ recipe: trimmed, result: EMPTY })
          useUiStore
            .getState()
            .pushToast({ kind: 'error', message: errorMessage(error) })
        })
    }, PREVIEW_DEBOUNCE_MS)
    return () => {
      live = false
      clearTimeout(timer)
    }
  }, [doc, trimmed])

  useEffect(() => {
    let live = true
    void (async () => {
      // Sequential on purpose: a legacy CJK repertoire resolves thousands of
      // names, and the small Latin rows should not wait behind it.
      for (const charset of CHARSETS) {
        try {
          const result = await previewRecipe(doc, charset.recipe)
          if (!live) return
          setCounts((current) => ({
            ...current,
            [charset.id]: {
              added: result.added.length,
              present: result.skipped.length,
            },
          }))
        } catch {
          // A repertoire the kernel cannot expand simply shows no counts.
          if (!live) return
        }
      }
    })()
    return () => {
      live = false
    }
  }, [doc])

  const close = () => setDialog(null)
  const cells: readonly GlyphCell[] = preview.added
  const canAdd = !busy && !pending && cells.length > 0

  const add = () => {
    if (!canAdd) return
    setBusy(true)
    void applyRecipe(doc, trimmed)
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
        useUiStore
          .getState()
          .pushToast({ kind: 'error', message: errorMessage(error) })
      })
  }

  return (
    <Dialog
      title={t('dialog.addGlyphs.title')}
      size="md"
      onCancel={close}
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
                    {count ? (
                      <>
                        <span>
                          {t('dialog.addGlyphs.new', { count: count.added })}
                        </span>
                        {count.present > 0 ? (
                          <span className={styles.badge}>
                            {t('dialog.addGlyphs.presentCount', {
                              count: count.present,
                            })}
                          </span>
                        ) : null}
                      </>
                    ) : (
                      <span>{t('dialog.addGlyphs.counting')}</span>
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
            <span>
              {t('dialog.addGlyphs.skipped', { count: preview.skipped.length })}
            </span>
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
