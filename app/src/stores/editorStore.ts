import { create } from 'zustand'

import type {
  DocId,
  Glyph,
  GlyphCell,
  GlyphId,
  Guide,
  LayerOutline,
  MasterId,
} from '../api'
import type { EditorPreview } from '../editor/editorTypes'
import { EMPTY_SELECTION, type Selection } from '../editor/selection'
import type { ToolId } from '../editor/tools/registry'
import type { GlyphScene } from '../views/glyph/glyphScene'
import type { GlyphOverlays } from '../views/glyph/renderGlyph'
import { DEFAULT_OVERLAYS } from '../views/glyph/renderGlyph'

/**
 * The glyph the mounted glyph view is showing. Published here so the shell's
 * inspector can render it without issuing a second `glyphs.detail` request.
 */
export interface ActiveGlyph {
  doc: DocId
  gid: GlyphId
  glyph: Glyph
  scene: GlyphScene
  cell: GlyphCell | null
  /** The editable layer, once `glyphs.outline` has answered. */
  outline: LayerOutline | null
  /** Master the shown layer belongs to. */
  master: MasterId | null
  /** That master's own guides: the "global" scope of the Guides panel. */
  masterGuides: readonly Guide[]
  /** Refetches the layer; every panel command reconciles through it. */
  reload: () => void
}

/** A pointer position in font units. */
export interface FontPoint {
  x: number
  y: number
}

/**
 * State of the glyph editor that other parts of the shell need to see: which
 * overlays are on (the View menu toggles them), where the pointer is (the
 * status bar prints it) and how far the canvas is zoomed (the toolbar shows
 * it). The viewport transform itself stays in the canvas, because it is a
 * mutable per-frame object rather than rendered state.
 */
export interface EditorState {
  overlays: GlyphOverlays
  pointer: FontPoint | null
  /** CSS pixels per font unit in the active glyph view. */
  zoom: number
  activeGlyph: ActiveGlyph | null
  /** The tool the tool strip shows as active. */
  tool: ToolId
  /** A tool borrowed by holding its key; overrides `tool` while held. */
  borrowedTool: ToolId | null
  /** What is selected on the canvas of the active glyph tab. */
  selection: Selection
  /** Whether the docked Info box is expanded. */
  infoBoxOpen: boolean
  /** Whether snapping is on; Ctrl suspends it for the duration of a gesture. */
  snapping: boolean
  /**
   * Whether an editing gesture is live. Data hooks stop reloading while it is,
   * because every `edit_apply` emits a document event and refetching the
   * outline mid-drag would fight the optimistic preview.
   */
  editing: boolean
  /**
   * The geometry a live gesture is showing, or `null` when none is running.
   * The Info box reads it so its numbers track the drag instead of lagging on
   * the last fetched outline.
   */
  preview: EditorPreview | null
  /**
   * The layer the Layers panel put on the canvas, or `null` for the master
   * layer of the selected master. Cleared whenever a different glyph opens,
   * because a layer id means nothing in another glyph.
   */
  editedLayer: string | null
  /**
   * Guides shown as measurement guides, keyed `scope:index`.
   *
   * The model has no measurement flag (`fw_model::Guide` is position, angle,
   * name, lock and colour), and this task adds no kernel field, so the choice
   * lives for the session only and is deliberately not persisted.
   */
  measurementGuides: ReadonlySet<string>
  setOverlay: (key: keyof GlyphOverlays, value: boolean) => void
  setActiveGlyph: (glyph: ActiveGlyph | null) => void
  toggleOverlay: (key: keyof GlyphOverlays) => void
  setPointer: (pointer: FontPoint | null) => void
  setZoom: (zoom: number) => void
  setTool: (tool: ToolId) => void
  borrowTool: (tool: ToolId | null) => void
  setSelection: (selection: Selection) => void
  setInfoBoxOpen: (open: boolean) => void
  toggleInfoBox: () => void
  setSnapping: (snapping: boolean) => void
  setEditing: (editing: boolean) => void
  setPreview: (preview: EditorPreview | null) => void
  setEditedLayer: (layer: string | null) => void
  toggleMeasurementGuide: (key: string) => void
}

/** The initial, action-free part of the editor state. */
export function initialEditorState(): Pick<
  EditorState,
  | 'overlays'
  | 'pointer'
  | 'zoom'
  | 'activeGlyph'
  | 'tool'
  | 'borrowedTool'
  | 'selection'
  | 'infoBoxOpen'
  | 'snapping'
  | 'editing'
  | 'preview'
  | 'editedLayer'
  | 'measurementGuides'
> {
  return {
    overlays: { ...DEFAULT_OVERLAYS },
    pointer: null,
    zoom: 1,
    activeGlyph: null,
    tool: 'select',
    borrowedTool: null,
    selection: EMPTY_SELECTION,
    infoBoxOpen: true,
    snapping: true,
    editing: false,
    preview: null,
    editedLayer: null,
    measurementGuides: new Set(),
  }
}

export const useEditorStore = create<EditorState>((set, get) => ({
  ...initialEditorState(),

  setOverlay: (key, value) =>
    set((state) => ({ overlays: { ...state.overlays, [key]: value } })),

  setActiveGlyph: (activeGlyph) => set({ activeGlyph }),

  toggleOverlay: (key) => get().setOverlay(key, !get().overlays[key]),

  setPointer: (pointer) => {
    const current = get().pointer
    if (
      current === pointer ||
      (current !== null &&
        pointer !== null &&
        current.x === pointer.x &&
        current.y === pointer.y)
    )
      return
    set({ pointer })
  },

  setZoom: (zoom) => {
    if (get().zoom === zoom) return
    set({ zoom })
  },

  setTool: (tool) => {
    if (get().tool === tool) return
    set({ tool })
  },

  borrowTool: (borrowedTool) => {
    if (get().borrowedTool === borrowedTool) return
    set({ borrowedTool })
  },

  setSelection: (selection) => {
    if (get().selection === selection) return
    set({ selection })
  },

  setInfoBoxOpen: (infoBoxOpen) => set({ infoBoxOpen }),

  toggleInfoBox: () => set((state) => ({ infoBoxOpen: !state.infoBoxOpen })),

  setSnapping: (snapping) => {
    if (get().snapping === snapping) return
    set({ snapping })
  },

  setEditing: (editing) => {
    if (get().editing === editing) return
    set({ editing })
  },

  setPreview: (preview) => {
    if (get().preview === preview) return
    set({ preview })
  },

  setEditedLayer: (editedLayer) => {
    if (get().editedLayer === editedLayer) return
    set({ editedLayer })
  },

  toggleMeasurementGuide: (key) =>
    set((state) => {
      const next = new Set(state.measurementGuides)
      if (!next.delete(key)) next.add(key)
      return { measurementGuides: next }
    }),
}))

/** The tool actually in effect: a borrowed tool wins over the chosen one. */
export function selectActiveTool(state: EditorState): ToolId {
  return state.borrowedTool ?? state.tool
}

/** The tool actually in effect, outside React. */
export function activeTool(): ToolId {
  return selectActiveTool(useEditorStore.getState())
}

/** Whether an overlay is currently shown, outside React. */
export function overlayEnabled(key: keyof GlyphOverlays): boolean {
  return useEditorStore.getState().overlays[key]
}
