/** Session-local annotation preferences and a single active path-focus scope. */
import { create } from 'zustand'
import type { AnnotationKind } from '../api'
import type { LayerTarget } from './pointCommands'
import { editorTargetKey } from './editorMetadata'

interface EditorMetadataState {
  focus: { owner: string; contours: ReadonlySet<number> } | null
  annotationKind: AnnotationKind
  annotationText: string
  focusPaths: (target: LayerTarget, contours: readonly number[]) => void
  clearFocus: () => void
  setAnnotationKind: (kind: AnnotationKind) => void
  setAnnotationText: (text: string) => void
}

export const useEditorMetadataStore = create<EditorMetadataState>((set) => ({
  focus: null,
  annotationKind: 'Text',
  annotationText: '',
  focusPaths: (target, contours) =>
    set({
      focus: contours.length
        ? { owner: editorTargetKey(target), contours: new Set(contours) }
        : null,
    }),
  clearFocus: () => set({ focus: null }),
  setAnnotationKind: (annotationKind) => set({ annotationKind }),
  setAnnotationText: (annotationText) => set({ annotationText }),
}))

/** A focus from a different glyph or layer never constrains this editor. */
export function focusedPaths(
  target: LayerTarget | null,
): ReadonlySet<number> | null {
  const focus = useEditorMetadataStore.getState().focus
  return target && focus?.owner === editorTargetKey(target)
    ? focus.contours
    : null
}
