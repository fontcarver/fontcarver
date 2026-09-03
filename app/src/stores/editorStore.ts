import { create } from 'zustand'

import type { DocId, Glyph, GlyphCell, GlyphId } from '../api'
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
  setOverlay: (key: keyof GlyphOverlays, value: boolean) => void
  setActiveGlyph: (glyph: ActiveGlyph | null) => void
  toggleOverlay: (key: keyof GlyphOverlays) => void
  setPointer: (pointer: FontPoint | null) => void
  setZoom: (zoom: number) => void
}

/** The initial, action-free part of the editor state. */
export function initialEditorState(): Pick<
  EditorState,
  'overlays' | 'pointer' | 'zoom' | 'activeGlyph'
> {
  return {
    overlays: { ...DEFAULT_OVERLAYS },
    pointer: null,
    zoom: 1,
    activeGlyph: null,
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
}))

/** Whether an overlay is currently shown, outside React. */
export function overlayEnabled(key: keyof GlyphOverlays): boolean {
  return useEditorStore.getState().overlays[key]
}
