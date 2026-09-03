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

import { useUiStore } from './uiStore'

/** What the glyph grid prints under each cell. */
export type LabelMode = 'name' | 'unicode' | 'index'

/** Smallest, largest and default glyph-cell edge length in CSS pixels. */
export const CELL_SIZE_MIN = 24
export const CELL_SIZE_MAX = 256
export const CELL_SIZE_DEFAULT = 64

/** Discrete cell sizes stepped through by the zoom commands. */
export const CELL_SIZE_STEPS = [
  24, 32, 40, 48, 64, 80, 96, 128, 160, 192, 224, 256,
] as const

/** A filter that matches every glyph. */
export function emptyFilter(): GlyphFilter {
  return {
    query: null,
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
  selection: ReadonlySet<GlyphId>
  filter: GlyphFilter
  sort: GlyphSort
  master: MasterId | null
  cellSize: number
  labelMode: LabelMode
}

/** A fresh view state for a newly opened document. */
export function newDocView(): DocView {
  return {
    sidebar: null,
    selection: new Set(),
    filter: emptyFilter(),
    sort: 'Order',
    master: null,
    cellSize: CELL_SIZE_DEFAULT,
    labelMode: 'name',
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
  stepCellSize: (doc: DocId, direction: 1 | -1) => void
  setLabelMode: (doc: DocId, mode: LabelMode) => void
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

  return {
    ...initialDocumentState(),

    ensure: (doc) => {
      if (!get().byDoc[doc]) patch(doc, {})
    },

    forget: (doc) =>
      set((state) => {
        if (!(doc in state.byDoc)) return state
        const byDoc = { ...state.byDoc }
        delete byDoc[doc]
        return { byDoc }
      }),

    loadSidebar: async (doc) => {
      try {
        patch(doc, { sidebar: await call('doc.sidebar', { doc }) })
      } catch (error) {
        useUiStore.getState().pushToast({
          kind: 'error',
          message: error instanceof Error ? error.message : String(error),
        })
      }
    },

    setFilter: (doc, update) => {
      const current = get().byDoc[doc]?.filter ?? emptyFilter()
      patch(doc, { filter: { ...current, ...update } })
    },

    setSort: (doc, sort) => patch(doc, { sort }),

    setMaster: (doc, master) => patch(doc, { master }),

    setCellSize: (doc, size) => patch(doc, { cellSize: clampCellSize(size) }),

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
