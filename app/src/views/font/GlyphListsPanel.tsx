import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { call, type DocId, type GlyphListView } from '../../api'
import { Button } from '../../components/Button'
import { Panel } from '../../components/Panel'
import {
  emptyFilter,
  selectDocView,
  useDocumentStore,
} from '../../stores/documentStore'
import { reportError, useUiStore } from '../../stores/uiStore'

import {
  glyphListFilter,
  isGlyphListActive,
  parseGlyphList,
} from './glyphLists'
import styles from './glyphLists.module.css'
import { glyphNamesOf } from './glyphNames'

interface Snapshot {
  doc: DocId
  generation: number
  lists: GlyphListView[]
}
interface Draft {
  doc: DocId
  id: string | null
  name: string
  names: string
}

/** Persistent, undoable name-based repertoires with missing-glyph previews. */
export function GlyphListsPanel({ doc }: { doc: DocId }) {
  const { t } = useTranslation()
  const view = useDocumentStore(selectDocView(doc))
  const setFilter = useDocumentStore((state) => state.setFilter)
  const setDialog = useUiStore((state) => state.setDialog)
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null)
  const [draft, setDraft] = useState<Draft | null>(null)
  const [busyDoc, setBusyDoc] = useState<DocId | null>(null)
  const [failedDoc, setFailedDoc] = useState<DocId | null>(null)
  const [retry, setRetry] = useState(0)
  const revision = useRef(0)
  const saved = snapshot?.doc === doc ? snapshot : null
  const editor = draft?.doc === doc ? draft : null
  const busy = busyDoc === doc
  const fresh = saved?.generation === view.generation
  const selected = glyphNamesOf(doc, view.selection)
  const names = parseGlyphList(editor?.names ?? '')
  const tooMany = names.length > 8192

  useEffect(() => {
    let cancelled = false
    const started = revision.current
    void call('inventory.get_lists', { doc }).then(
      ({ lists }) => {
        if (cancelled || started !== revision.current) return
        setSnapshot({ doc, generation: view.generation, lists })
        setFailedDoc(null)
      },
      (error: unknown) => {
        if (cancelled || started !== revision.current) return
        setFailedDoc(doc)
        reportError(error)
      },
    )
    return () => {
      cancelled = true
    }
  }, [doc, view.generation, retry])

  const persist = async (remove: boolean) => {
    if (!editor || !saved || busy || !fresh) return
    const edited = saved.lists.find((list) => list.id === editor.id)
    const lists = saved.lists
      .filter((list) => list.id !== editor.id)
      .map(({ id, name, glyphs }) => ({ id, name, glyphs }))
    if (!remove) {
      const list = {
        id: editor.id ?? crypto.randomUUID(),
        name: editor.name.trim(),
        glyphs: names,
      }
      const position = edited ? saved.lists.indexOf(edited) : lists.length
      lists.splice(position, 0, list)
    }
    revision.current += 1
    setBusyDoc(doc)
    try {
      const response = await call('inventory.set_lists', { doc, lists })
      const generation =
        useDocumentStore.getState().byDoc[doc]?.generation ?? view.generation
      setSnapshot((current) =>
        current?.doc === doc
          ? { doc, generation, lists: response.lists }
          : current,
      )
      setDraft((current) => (current?.doc === doc ? null : current))
      if (
        edited &&
        isGlyphListActive(
          useDocumentStore.getState().byDoc[doc]?.filter ?? emptyFilter(),
          edited.glyphs,
        )
      ) {
        setFilter(doc, remove ? emptyFilter() : glyphListFilter(names))
      }
    } catch (error) {
      reportError(error)
    } finally {
      setBusyDoc((current) => (current === doc ? null : current))
    }
  }

  return (
    <Panel title={t('glyphLists.title')}>
      <div className={styles.lists}>
        {saved === null ? (
          <p className={styles.hint}>
            {t(
              failedDoc === doc
                ? 'glyphLists.loadFailed'
                : 'glyphLists.loading',
            )}
          </p>
        ) : saved.lists.length === 0 ? (
          <p className={styles.hint}>{t('glyphLists.empty')}</p>
        ) : (
          saved.lists.map((list) => {
            const active = isGlyphListActive(view.filter, list.glyphs)
            return (
              <div key={list.id} className={styles.entry}>
                <div className={styles.row}>
                  <button
                    type="button"
                    className={styles.filter}
                    aria-pressed={active}
                    onClick={() =>
                      setFilter(
                        doc,
                        active ? emptyFilter() : glyphListFilter(list.glyphs),
                      )
                    }
                  >
                    <span className={styles.name}>{list.name}</span>
                    <span className={styles.count}>
                      {t('glyphLists.coverage', {
                        present: list.present.length,
                        total: list.glyphs.length,
                      })}
                    </span>
                  </button>
                  <Button
                    variant="ghost"
                    disabled={busy}
                    aria-label={t('glyphLists.editNamed', { name: list.name })}
                    onClick={() =>
                      setDraft({
                        doc,
                        id: list.id,
                        name: list.name,
                        names: list.glyphs.join('\n'),
                      })
                    }
                  >
                    {t('glyphLists.edit')}
                  </Button>
                </div>
                {list.missing.length > 0 && (
                  <Button
                    disabled={!fresh || busy}
                    className={styles.generate}
                    onClick={() =>
                      setDialog({
                        kind: 'addGlyphs',
                        doc,
                        recipe: list.missing.join(' '),
                      })
                    }
                  >
                    {t('glyphLists.generateMissing', {
                      count: list.missing.length,
                    })}
                  </Button>
                )}
              </div>
            )
          })
        )}
        {failedDoc === doc && (
          <Button onClick={() => setRetry((value) => value + 1)}>
            {t('glyphLists.retry')}
          </Button>
        )}
        {!editor ? (
          <Button
            disabled={!saved || busy || saved.lists.length >= 128}
            onClick={() => setDraft({ doc, id: null, name: '', names: '' })}
          >
            {t('glyphLists.new')}
          </Button>
        ) : (
          <form
            className={styles.editor}
            onSubmit={(event) => {
              event.preventDefault()
              void persist(false)
            }}
          >
            <label>
              {t('glyphLists.name')}
              <input
                value={editor.name}
                maxLength={128}
                disabled={busy}
                onChange={(event) =>
                  setDraft({ ...editor, name: event.target.value })
                }
              />
            </label>
            <label>
              {t('glyphLists.names')}
              <textarea
                value={editor.names}
                rows={5}
                disabled={busy}
                onChange={(event) =>
                  setDraft({ ...editor, names: event.target.value })
                }
              />
            </label>
            <p className={styles.hint}>{t('glyphLists.importHint')}</p>
            <Button
              disabled={busy || selected.length === 0}
              onClick={() =>
                setDraft({ ...editor, names: selected.join('\n') })
              }
            >
              {t('glyphLists.useSelection', { count: selected.length })}
            </Button>
            {tooMany && (
              <p role="alert" className={styles.hint}>
                {t('glyphLists.tooMany', { count: 8192 })}
              </p>
            )}
            <div className={styles.actions}>
              <Button
                type="submit"
                variant="primary"
                disabled={busy || !fresh || !editor.name.trim() || tooMany}
              >
                {t('common.save')}
              </Button>
              <Button disabled={busy} onClick={() => setDraft(null)}>
                {t('common.cancel')}
              </Button>
              {editor.id !== null && (
                <Button
                  disabled={busy || !fresh}
                  onClick={() => void persist(true)}
                >
                  {t('glyphLists.delete')}
                </Button>
              )}
            </div>
          </form>
        )}
      </div>
    </Panel>
  )
}
