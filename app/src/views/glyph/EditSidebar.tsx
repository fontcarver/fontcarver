import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'

import type { DocId } from '../../api'
import { selectDocView, useDocumentStore } from '../../stores/documentStore'
import { selectSummary, useSessionStore } from '../../stores/sessionStore'
import { useEditorStore } from '../../stores/editorStore'
import type { GlyphQuery } from '../font/glyphSelection'

import { EditGlyphList } from './EditGlyphList'
import styles from './editSidebar.module.css'
import { LayerList } from './panels/LayerList'

export interface EditSidebarProps {
  doc: DocId
}

/**
 * The left pane of an Edit tab (design spec §3): the layers of the glyph on
 * top, the Font view's current glyph list below. Both are the same components
 * the inspector and the Font view use, so the three surfaces cannot disagree
 * about what a layer is called or which glyphs the filter matches.
 */
export function EditSidebar({ doc }: EditSidebarProps) {
  const { t } = useTranslation()
  const summary = useSessionStore(selectSummary(doc))
  const view = useDocumentStore(selectDocView(doc))
  const active = useEditorStore((state) => state.activeGlyph)
  const masters = summary?.masters

  const query = useMemo<GlyphQuery>(
    () => ({
      doc,
      master: view.master ?? masters?.[0]?.id ?? null,
      filter: view.filter,
      sort: view.sort,
    }),
    [doc, masters, view.filter, view.master, view.sort],
  )
  // The page cache inside the list belongs to one query, so a new filter,
  // sort or master mounts a fresh list rather than re-using stale pages.
  const queryKey = useMemo(
    () =>
      JSON.stringify([query.doc, query.master ?? '', query.sort, query.filter]),
    [query],
  )

  if (!summary) return <p className={styles.note}>{t('inspector.empty')}</p>

  return (
    <div className={styles.sidebar}>
      <div className={styles.layers}>
        <h2 className={styles.heading}>{t('panel.sidebar.layers')}</h2>
        {active?.outline ? (
          <LayerList
            doc={doc}
            layers={active.glyph.layers}
            masters={summary.masters}
            current={active.outline.layer}
          />
        ) : (
          <p className={styles.note}>{t('glyphView.loading')}</p>
        )}
      </div>
      <div className={styles.glyphs}>
        <h2 className={styles.glyphsHeading}>{t('panel.sidebar.glyphs')}</h2>
        {active ? (
          <EditGlyphList
            key={queryKey}
            summary={summary}
            gid={active.gid}
            query={query}
          />
        ) : (
          <p className={styles.note}>{t('glyphView.loading')}</p>
        )}
      </div>
      <p className={styles.note}>{t('panel.sidebar.filterHint')}</p>
    </div>
  )
}
