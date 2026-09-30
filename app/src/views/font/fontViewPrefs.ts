/**
 * How the font view is presented, kept across reloads and windows.
 *
 * Grid or list, cell size, what the cell label shows, which list columns appear
 * and in what order, and whether an empty cell may show the character from a
 * system face. These are workspace preferences, not document data: they belong
 * to the person, not to the font, so they live in `localStorage` next to the
 * other interface preferences (`uiStore`, `workspaceStore`, `keymapStore`) and
 * never in the `.fcf`.
 *
 * The document store stays the live state for the three settings that other
 * parts of the shell change (the zoom commands step the cell size, for
 * instance); this module mirrors them into storage and applies the stored value
 * when a document's view is created. The list column set has no home in the
 * document store, so it lives here outright.
 */

import { create } from 'zustand'

import type { DocId } from '../../api'
import {
  CELL_SIZE_MAX,
  CELL_SIZE_MIN,
  useDocumentStore,
  type GlyphViewMode,
  type LabelMode,
} from '../../stores/documentStore'

import {
  DEFAULT_GLYPH_TABLE_COLUMNS,
  GLYPH_TABLE_COLUMNS,
  type GlyphTableColumn,
} from './glyphTableColumns'

/** Storage shared by reloads and by every window of this app origin. */
export const FONT_VIEW_PREFS_KEY = 'fontcarver.fontView.v1'

/** The presentation settings of the font view. */
export interface FontViewPrefs {
  viewMode: GlyphViewMode
  cellSize: number
  labelMode: LabelMode
  /** Visible list columns, in display order; never empty. */
  tableColumns: readonly GlyphTableColumn[]
  /**
   * Whether an empty cell may show its character drawn with a system face.
   *
   * On by default: it is what makes a 20,000-cell CJK font navigable before any
   * glyph is drawn. Turning it off leaves empty cells genuinely empty.
   */
  referenceCharacters: boolean
}

const DEFAULTS: FontViewPrefs = {
  viewMode: 'grid',
  cellSize: 64,
  labelMode: 'name',
  tableColumns: DEFAULT_GLYPH_TABLE_COLUMNS,
  referenceCharacters: true,
}

function clampCellSize(size: number): number {
  if (!Number.isFinite(size)) return DEFAULTS.cellSize
  return Math.min(CELL_SIZE_MAX, Math.max(CELL_SIZE_MIN, Math.round(size)))
}

/** Keeps only known column names, in the stored order, without duplicates. */
function readColumns(value: unknown): readonly GlyphTableColumn[] {
  if (!Array.isArray(value)) return DEFAULTS.tableColumns
  const seen = new Set<GlyphTableColumn>()
  for (const entry of value) {
    if (GLYPH_TABLE_COLUMNS.includes(entry as GlyphTableColumn)) {
      seen.add(entry as GlyphTableColumn)
    }
  }
  // A stored set that lost every known column would leave an unusable table.
  return seen.size === 0 ? DEFAULTS.tableColumns : [...seen]
}

/** Corrupt or partial storage falls back to the shipped presentation. */
export function readFontViewPrefs(): FontViewPrefs {
  try {
    const text = globalThis.localStorage?.getItem(FONT_VIEW_PREFS_KEY)
    if (!text) return DEFAULTS
    const stored = JSON.parse(text) as Partial<FontViewPrefs>
    return {
      viewMode: stored.viewMode === 'list' ? 'list' : 'grid',
      cellSize: clampCellSize(stored.cellSize ?? DEFAULTS.cellSize),
      labelMode:
        stored.labelMode === 'unicode' || stored.labelMode === 'index'
          ? stored.labelMode
          : 'name',
      tableColumns: readColumns(stored.tableColumns),
      referenceCharacters:
        stored.referenceCharacters ?? DEFAULTS.referenceCharacters,
    }
  } catch {
    return DEFAULTS
  }
}

function write(prefs: FontViewPrefs): void {
  try {
    globalThis.localStorage?.setItem(FONT_VIEW_PREFS_KEY, JSON.stringify(prefs))
  } catch {
    /* Storage is unavailable (private mode, disabled cookies): stay in memory. */
  }
}

/** The persisted presentation plus the actions that change it. */
export interface FontViewPrefsState extends FontViewPrefs {
  /** Replaces the visible column set and order, ignoring an empty set. */
  setTableColumns: (columns: readonly GlyphTableColumn[]) => void
  /** Moves one column immediately before another. */
  moveColumn: (column: GlyphTableColumn, before: GlyphTableColumn) => void
  /** Shows or hides one column; the last visible column cannot be hidden. */
  toggleColumn: (column: GlyphTableColumn) => void
  setReferenceCharacters: (enabled: boolean) => void
  /** Records a presentation change the document store owns. */
  remember: (patch: Partial<FontViewPrefs>) => void
}

export const useFontViewPrefs = create<FontViewPrefsState>((set, get) => {
  const commit = (patch: Partial<FontViewPrefs>): void => {
    const next: FontViewPrefs = {
      viewMode: patch.viewMode ?? get().viewMode,
      cellSize: clampCellSize(patch.cellSize ?? get().cellSize),
      labelMode: patch.labelMode ?? get().labelMode,
      tableColumns: patch.tableColumns ?? get().tableColumns,
      referenceCharacters:
        patch.referenceCharacters ?? get().referenceCharacters,
    }
    write(next)
    set(next)
  }
  return {
    ...readFontViewPrefs(),
    setTableColumns: (columns) => {
      const kept = columns.filter((column) =>
        GLYPH_TABLE_COLUMNS.includes(column),
      )
      if (kept.length === 0) return
      commit({ tableColumns: [...new Set(kept)] })
    },
    moveColumn: (column, before) => {
      const current = get().tableColumns
      if (column === before || !current.includes(before)) return
      const rest = current.filter((entry) => entry !== column)
      rest.splice(rest.indexOf(before), 0, column)
      commit({ tableColumns: rest })
    },
    toggleColumn: (column) => {
      const current = get().tableColumns
      if (current.includes(column)) {
        if (current.length === 1) return
        commit({ tableColumns: current.filter((entry) => entry !== column) })
        return
      }
      // A column reappears in the canonical order, not at the end.
      commit({
        tableColumns: GLYPH_TABLE_COLUMNS.filter(
          (entry) => entry === column || current.includes(entry),
        ),
      })
    },
    setReferenceCharacters: (referenceCharacters) =>
      commit({ referenceCharacters }),
    remember: commit,
  }
})

/** Documents whose view state already received the stored presentation. */
const applied = new Set<DocId>()

/**
 * Seeds one document's view state with the stored presentation, once.
 *
 * Called when the font view mounts. A document opened later in the same session
 * gets the same presentation, and a change made in one document becomes the
 * starting point for the next. A field something already chose for this
 * document — a command that opened it straight into list mode, say — is left
 * alone: the stored value seeds a fresh view, it does not overrule a decision.
 */
export function applyFontViewPrefs(doc: DocId): void {
  if (applied.has(doc)) return
  applied.add(doc)
  const prefs = useFontViewPrefs.getState()
  const store = useDocumentStore.getState()
  const view = store.byDoc[doc]
  if (view === undefined || view.viewMode === DEFAULTS.viewMode) {
    store.setViewMode(doc, prefs.viewMode)
  }
  if (view === undefined || view.cellSize === DEFAULTS.cellSize) {
    store.setCellSize(doc, prefs.cellSize)
  }
  if (view === undefined || view.labelMode === DEFAULTS.labelMode) {
    store.setLabelMode(doc, prefs.labelMode)
  }
}

/** Forgets that a document was initialised; used by tests and on close. */
export function forgetFontViewPrefs(doc?: DocId): void {
  if (doc === undefined) applied.clear()
  else applied.delete(doc)
}

/**
 * Mirrors the document store's presentation fields into storage.
 *
 * Subscribed once per module load: the zoom commands, the toolbar and the view
 * menu all go through the document store, and every one of them should be
 * remembered without each caller having to think about it.
 */
useDocumentStore.subscribe((state) => {
  for (const doc of Object.keys(state.byDoc)) {
    const view = state.byDoc[Number(doc) as DocId]
    if (!view || !applied.has(Number(doc) as DocId)) continue
    const prefs = useFontViewPrefs.getState()
    if (
      view.viewMode === prefs.viewMode &&
      view.cellSize === prefs.cellSize &&
      view.labelMode === prefs.labelMode
    ) {
      continue
    }
    prefs.remember({
      viewMode: view.viewMode,
      cellSize: view.cellSize,
      labelMode: view.labelMode,
    })
    return
  }
})
