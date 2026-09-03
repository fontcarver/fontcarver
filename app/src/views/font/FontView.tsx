import { useMemo } from 'react'

import type { DocSummary } from '../../api'
import { selectDocView, useDocumentStore } from '../../stores/documentStore'

import './caches'
import { FilterChips } from './FilterChips'
import styles from './fontView.module.css'
import { GlyphGrid } from './GlyphGrid'
import type { GlyphQuery } from './glyphSelection'
import { GridToolbar } from './GridToolbar'

export interface FontViewProps {
  summary: DocSummary
}

/** The Font tab: display controls, active filter chips and the glyph grid. */
export function FontView({ summary }: FontViewProps) {
  const doc = summary.doc
  const view = useDocumentStore(selectDocView(doc))
  const master = view.master ?? summary.masters[0]?.id ?? null

  const query = useMemo<GlyphQuery>(
    () => ({ doc, master, filter: view.filter, sort: view.sort }),
    [doc, master, view.filter, view.sort],
  )
  const queryKey = useMemo(
    () =>
      JSON.stringify([query.doc, query.master ?? '', query.sort, query.filter]),
    [query],
  )

  return (
    <div className={styles.view}>
      <GridToolbar summary={summary} view={view} />
      <FilterChips doc={doc} filter={view.filter} />
      <GlyphGrid key={queryKey} summary={summary} query={query} />
    </div>
  )
}
