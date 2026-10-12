import { create } from 'zustand'
import type { DocId, VariationPreview } from '../api'

/** Nonpersistent interpolation feedback belonging to one editor panel. */
export interface CanvasVariationPreview {
  doc: DocId
  glyph: string
  path: string
  width: number
}

interface VariationPreviewState {
  preview: CanvasVariationPreview | null
  show: (
    doc: DocId,
    glyph: string,
    value: Pick<VariationPreview, 'path' | 'width'>,
  ) => void
  clear: (doc: DocId, glyph: string) => void
}

/** Axis scrubbing never writes to document or editor selection state. */
export const useVariationPreviewStore = create<VariationPreviewState>(
  (set) => ({
    preview: null,
    show: (doc, glyph, value) =>
      set({ preview: { doc, glyph, path: value.path, width: value.width } }),
    clear: (doc, glyph) =>
      set((state) =>
        state.preview?.doc === doc && state.preview.glyph === glyph
          ? { preview: null }
          : state,
      ),
  }),
)
