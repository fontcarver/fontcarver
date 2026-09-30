import { create } from 'zustand'

import {
  call,
  type DocId,
  type GlyphFilter,
  type GlyphId,
  type GlyphSort,
  type MasterId,
  type SidebarCounts,
} from '../api'

import { reportError } from './uiStore'

/** What the glyph grid prints under each cell. */
export type LabelMode = 'name' | 'unicode' | 'index'

/** Presentation used by the font overview. */
export type GlyphViewMode = 'grid' | 'list'

/** Smallest, largest and default glyph-cell edge length in CSS pixels. */
export const CELL_SIZE_MIN = 24
export const CELL_SIZE_MAX = 256
export const CELL_SIZE_DEFAULT = 64

/** Discrete cell sizes stepped through by the zoom commands. */
export const CELL_SIZE_STEPS = [
  24, 32, 40, 48, 64, 80, 96, 128, 160, 192, 224, 256,
] as const

/** Whether any condition of a filter narrows the glyph list. */
export function isFiltered(filter: GlyphFilter): boolean {
  return (
    Boolean(filter.query) ||
    Boolean(filter.smart_expression?.trim()) ||
    filter.glyph_names != null ||
    filter.categories.length > 0 ||
    filter.scripts.length > 0 ||
    filter.blocks.length > 0 ||
    filter.color_labels.length > 0 ||
    filter.tags.length > 0 ||
    filter.unicode_ranges.length > 0 ||
    filter.unencoded_only ||
    filter.changed_only ||
    filter.has_components !== null ||
    filter.cjk != null
  )
}

/** A filter that matches every glyph. */
export function emptyFilter(): GlyphFilter {
  return {
    query: null,
    query_scope: 'All',
    case_sensitive: false,
    regex: false,
    smart_expression: null,
    glyph_names: null,
    cjk: null,
    categories: [],
    scripts: [],
    blocks: [],
    unencoded_only: false,
    color_labels: [],
    tags: [],
    unicode_ranges: [],
    has_components: null,
    changed_only: false,
  }
}

/** Per-document font-view state. */
export interface DocView {
  sidebar: SidebarCounts | null
  /** Document generation the sidebar counts were computed for. */
  sidebarGeneration: number | null
  /** Latest document generation reported by a kernel event. */
  generation: number
  /** Glyphs matching the current filter, or `null` until the grid knows. */
  matchCount: number | null
  selection: ReadonlySet<GlyphId>
  /** Cell the grid's keyboard focus is on; what `glyph.open` acts upon. */
  focus: GlyphId | null
  filter: GlyphFilter
  sort: GlyphSort
  master: MasterId | null
  cellSize: number
  labelMode: LabelMode
  viewMode: GlyphViewMode
}

/** A fresh view state for a newly opened document. */
export function newDocView(): DocView {
  return {
    sidebar: null,
    sidebarGeneration: null,
    generation: 0,
    matchCount: null,
    selection: new Set(),
    focus: null,
    filter: emptyFilter(),
    sort: 'Order',
    master: null,
    cellSize: CELL_SIZE_DEFAULT,
    labelMode: 'name',
    viewMode: 'grid',
  }
}

/**
 * Stable placeholder returned for documents with no view state yet. Frozen so a
 * selector can return it on every render without causing update loops.
 */
export const EMPTY_DOC_VIEW: DocView = Object.freeze({
  ...newDocView(),
  selection: Object.freeze(new Set<GlyphId>()) as ReadonlySet<GlyphId>,
})

/** Called with the document id when a document's view state is discarded. */
export type ForgetListener = (doc: DocId) => void

const forgetListeners = new Set<ForgetListener>()

/**
 * Subscribes to document teardown. Per-document caches outside this store
 * (glyph names, master metrics) register here so closing a document releases
 * them without the store having to know what they are.
 */
export function onDocumentForgotten(listener: ForgetListener): () => void {
  forgetListeners.add(listener)
  return () => {
    forgetListeners.delete(listener)
  }
}

function clampCellSize(size: number): number {
  return Math.min(CELL_SIZE_MAX, Math.max(CELL_SIZE_MIN, Math.round(size)))
}

/** View state for every open document, keyed by document id. */
export interface DocumentState {
  byDoc: Record<DocId, DocView>
  ensure: (doc: DocId) => void
  forget: (doc: DocId) => void
  loadSidebar: (doc: DocId) => Promise<void>
  setFilter: (doc: DocId, patch: Partial<GlyphFilter>) => void
  setSort: (doc: DocId, sort: GlyphSort) => void
  setMaster: (doc: DocId, master: MasterId | null) => void
  setCellSize: (doc: DocId, size: number) => void
  setMatchCount: (doc: DocId, matchCount: number | null) => void
  setFocus: (doc: DocId, focus: GlyphId | null) => void
  noteGeneration: (doc: DocId, generation: number) => void
  stepCellSize: (doc: DocId, direction: 1 | -1) => void
  setLabelMode: (doc: DocId, mode: LabelMode) => void
  setViewMode: (doc: DocId, mode: GlyphViewMode) => void
  setSelection: (doc: DocId, selection: Iterable<GlyphId>) => void
  toggleSelection: (doc: DocId, gid: GlyphId) => void
  clearSelection: (doc: DocId) => void
}

/** Creates the initial, action-free part of the document state. */
export function initialDocumentState(): Pick<DocumentState, 'byDoc'> {
  return { byDoc: {} }
}

export const useDocumentStore = create<DocumentState>((set, get) => {
  const patch = (doc: DocId, update: Partial<DocView>): void => {
    set((state) => ({
      byDoc: {
        ...state.byDoc,
        [doc]: { ...(state.byDoc[doc] ?? newDocView()), ...update },
      },
    }))
  }

  /**
   * Applies an update only while the document is still open. Responses that
   * arrive after the document was closed must not resurrect its view state.
   */
  const patchExisting = (doc: DocId, update: Partial<DocView>): boolean => {
    const current = get().byDoc[doc]
    if (!current) return false
    set((state) => ({
      byDoc: { ...state.byDoc, [doc]: { ...current, ...update } },
    }))
    return true
  }

  return {
    ...initialDocumentState(),

    ensure: (doc) => {
      if (!get().byDoc[doc]) patch(doc, {})
    },

    forget: (doc) => {
      for (const listener of [...forgetListeners]) listener(doc)
      set((state) => {
        if (!(doc in state.byDoc)) return state
        const byDoc = { ...state.byDoc }
        delete byDoc[doc]
        return { byDoc }
      })
    },

    loadSidebar: async (doc) => {
      get().ensure(doc)
      // Stamp the request with the generation it was issued for, so a later
      // edit is seen as making the counts stale again.
      const generation = get().byDoc[doc]?.generation ?? 0
      try {
        const sidebar = await call('doc.sidebar', { doc })
        patchExisting(doc, { sidebar, sidebarGeneration: generation })
      } catch (error) {
        // A document closed mid-request fails by design; stay quiet then.
        if (!get().byDoc[doc]) return
        reportError(error)
      }
    },

    setFilter: (doc, update) => {
      const current = get().byDoc[doc]?.filter ?? emptyFilter()
      patch(doc, { filter: { ...current, ...update }, matchCount: null })
    },

    setSort: (doc, sort) => patch(doc, { sort }),

    setMaster: (doc, master) => patch(doc, { master }),

    setCellSize: (doc, size) => patch(doc, { cellSize: clampCellSize(size) }),

    setMatchCount: (doc, matchCount) => {
      if (get().byDoc[doc]?.matchCount === matchCount) return
      patch(doc, { matchCount })
    },

    setFocus: (doc, focus) => {
      if (get().byDoc[doc]?.focus === focus) return
      patch(doc, { focus })
    },

    /**
     * Records the document generation a kernel event reported. Only documents
     * that already have view state are updated: an event for a document nobody
     * is looking at must not create one.
     */
    noteGeneration: (doc, generation) => {
      const current = get().byDoc[doc]
      if (!current || current.generation >= generation) return
      patchExisting(doc, { generation })
    },

    stepCellSize: (doc, direction) => {
      const current = get().byDoc[doc]?.cellSize ?? CELL_SIZE_DEFAULT
      const steps = CELL_SIZE_STEPS
      const index = steps.findIndex((step) => step >= current)
      const from = index < 0 ? steps.length - 1 : index
      const next =
        steps[Math.min(steps.length - 1, Math.max(0, from + direction))]
      patch(doc, { cellSize: clampCellSize(next ?? current) })
    },

    setLabelMode: (doc, labelMode) => patch(doc, { labelMode }),

    setViewMode: (doc, viewMode) => patch(doc, { viewMode }),

    setSelection: (doc, selection) =>
      patch(doc, { selection: new Set(selection) }),

    toggleSelection: (doc, gid) => {
      const selection = new Set(get().byDoc[doc]?.selection ?? [])
      if (selection.has(gid)) selection.delete(gid)
      else selection.add(gid)
      patch(doc, { selection })
    },

    clearSelection: (doc) => patch(doc, { selection: new Set() }),
  }
})

/** Selector factory: view state for one document, or a shared empty view. */
export function selectDocView(doc: DocId | null) {
  return (state: DocumentState): DocView =>
    doc === null ? EMPTY_DOC_VIEW : (state.byDoc[doc] ?? EMPTY_DOC_VIEW)
}
