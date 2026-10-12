import { useCallback, useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { call, type DocId, type DocumentHistory } from '../api'
import { withSettledDocument } from '../editor/documentPersistence'
import { selectDocView, useDocumentStore } from '../stores/documentStore'
import { selectSummary, useSessionStore } from '../stores/sessionStore'
import { reportError, useUiStore } from '../stores/uiStore'
import { PanelSection } from '../views/glyph/panels/PanelSection'

import styles from './HistoryPanel.module.css'

/** Stable id under which the Inspector remembers this section's state. */
const SECTION = 'history'

/** Translation keys for the roles the kernel reports for an unnamed layer. */
const LAYER_ROLES = [
  'master',
  'background',
  'intermediate',
  'alternate',
  'smart',
  'backup',
  'other',
] as const

/**
 * A layer's label: its own name, or its role in the reader's language.
 *
 * The kernel never sends prose, only a role identifier, so the wording lives
 * here. An identifier this build does not know falls back to "other" rather
 * than printing the raw string.
 */
function layerLabel(
  layer: DocumentHistory['entries'][number]['layers'][number],
  t: (key: string) => string,
): string {
  if (layer.name) return layer.name
  const role = LAYER_ROLES.find((known) => known === layer.role) ?? 'other'
  return t(`history.layerRole.${role}`)
}

/** The scope sentence shown under one entry's label. */
function scopeText(
  entry: DocumentHistory['entries'][number],
  t: (key: string, options?: Record<string, unknown>) => string,
): string {
  const parts: string[] = []
  const list = (names: readonly string[]) =>
    names.length > 3
      ? `${names.slice(0, 3).join(', ')} ${t('history.more', {
          count: names.length - 3,
        })}`
      : names.join(', ')
  if (entry.glyphs.length > 0)
    parts.push(t('history.scopeGlyphs', { names: list(entry.glyphs) }))
  if (entry.layers.length > 0)
    parts.push(
      t('history.scopeLayers', {
        names: list(entry.layers.map((layer) => layerLabel(layer, t))),
      }),
    )
  if (entry.strikes.length > 0)
    parts.push(t('history.scopeStrikes', { names: list(entry.strikes) }))
  if (entry.font_level) parts.push(t('history.scopeFont'))
  return parts.join(' · ')
}

/**
 * The document's undo timeline, newest first.
 *
 * Every row is one gesture, labelled with what it changed and where: a glyph, a
 * layer inside it, a bitmap strike, or font-wide data. Selecting a row moves the
 * document to the state just after that operation by replaying undo or redo, so
 * it produces exactly the same result as pressing the shortcut that many times.
 *
 * The list states the length it was built from. If the timeline has moved on —
 * another window edited the same document, or the memory budget dropped an old
 * entry — the kernel refuses the jump and the list reloads instead of taking the
 * document somewhere the user did not point at.
 */
export function HistoryPanel({ doc }: { doc: DocId }) {
  const { t } = useTranslation()
  // The timeline is read only while the section is open, so a document nobody
  // is inspecting costs one request per gesture less.
  const open = useUiStore((state) => state.sections[SECTION] ?? false)
  const summary = useSessionStore(selectSummary(doc))
  const view = useDocumentStore(selectDocView(doc))
  const [history, setHistory] = useState<DocumentHistory | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  // Any edit, undo or redo bumps one of these, so the list follows the document
  // without polling and without a second invalidation channel of its own.
  const revision = `${view.generation}:${summary?.undo_label ?? ''}:${
    summary?.redo_label ?? ''
  }`

  const load = useCallback(
    (signal: { cancelled: boolean }) => {
      void call('doc.history', { doc }).then(
        (result) => {
          if (!signal.cancelled) setHistory(result)
        },
        (error: unknown) => {
          if (!signal.cancelled) reportError(error)
        },
      )
    },
    [doc],
  )

  useEffect(() => {
    if (!open) return
    const signal = { cancelled: false }
    load(signal)
    return () => {
      signal.cancelled = true
    }
  }, [load, open, revision])

  const goTo = async (cursor: number) => {
    if (busy || !history) return
    setBusy(true)
    setNotice(null)
    try {
      const next = await withSettledDocument(doc, () =>
        call('doc.history_go', { doc, cursor, expected_len: history.len }),
      )
      setHistory(next)
      await useSessionStore.getState().refreshSummary(doc)
    } catch (error) {
      // A stale list is the expected failure here, and it is recoverable: say
      // so once and show the timeline as it actually is.
      setNotice(t('history.stale'))
      const signal = { cancelled: false }
      load(signal)
      reportError(error)
    } finally {
      setBusy(false)
    }
  }

  const entries = history ? [...history.entries].reverse() : []
  const atStart = history?.cursor === 0

  return (
    <PanelSection
      id={SECTION}
      title={t('history.title')}
      count={history?.len}
      defaultOpen={false}
    >
      {history === null ? null : history.entries.length === 0 ? (
        <p className={styles.empty}>{t('history.empty')}</p>
      ) : (
        <>
          {history.truncated ? (
            <p className={styles.empty}>{t('history.truncated')}</p>
          ) : null}
          <ul className={styles.list} aria-label={t('history.list')}>
            {entries.map((entry) => {
              const current = history.cursor === entry.index + 1
              const scope = scopeText(entry, t)
              return (
                <li key={entry.index}>
                  <button
                    type="button"
                    disabled={busy}
                    aria-current={current ? 'true' : undefined}
                    aria-label={t('history.jumpTo', { label: entry.label })}
                    className={`${styles.entry} ${
                      entry.undone ? styles.undone : ''
                    } ${current ? styles.current : ''}`}
                    onClick={() => void goTo(entry.index + 1)}
                  >
                    <span className={styles.label}>{entry.label}</span>
                    {scope ? (
                      <span className={styles.scope}>{scope}</span>
                    ) : null}
                    {current ? (
                      <span className={styles.marker}>
                        {t('history.currentMarker')}
                      </span>
                    ) : entry.undone ? (
                      <span className={styles.marker}>
                        {t('history.undone')}
                      </span>
                    ) : null}
                  </button>
                </li>
              )
            })}
            <li>
              <button
                type="button"
                disabled={busy || atStart}
                aria-current={atStart ? 'true' : undefined}
                aria-label={t('history.jumpToStart')}
                className={`${styles.entry} ${atStart ? styles.current : ''}`}
                onClick={() => void goTo(0)}
              >
                <span className={styles.label}>{t('history.start')}</span>
                {atStart ? (
                  <span className={styles.marker}>
                    {t('history.currentMarker')}
                  </span>
                ) : null}
              </button>
            </li>
          </ul>
        </>
      )}
      {notice ? (
        <p role="status" className={styles.empty}>
          {notice}
        </p>
      ) : null}
    </PanelSection>
  )
}
