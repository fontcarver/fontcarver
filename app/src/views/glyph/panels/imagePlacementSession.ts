/**
 * The one layer whose source image is being placed on the canvas.
 *
 * The Source image section owns the document revision and the write, and the
 * canvas owns the pointer and the handles, so the two meet here: the section
 * publishes which layer is being placed and how to commit a transform, and the
 * canvas reads it. Only one layer can be in placement mode at a time, which is
 * what keeps a handle from writing to a layer that is no longer on screen.
 */

import { create } from 'zustand'

import type { Affine } from '../../../editor/affine'

/** A layer that has offered its placement to the canvas. */
export interface ImagePlacementSession {
  /** `editorTargetKey` of the layer being placed. */
  owner: string
  /** Whether the image or its glyph refuses edits. */
  locked: boolean
  /** Writes one finished gesture as a single undo entry. */
  commit: (transform: Affine) => void
}

interface ImagePlacementStore {
  session: ImagePlacementSession | null
  /** Offers a layer's placement to the canvas, replacing any earlier offer. */
  open: (session: ImagePlacementSession) => void
  /** Withdraws `owner`'s offer; a different owner's offer is left alone. */
  close: (owner: string) => void
}

export const useImagePlacement = create<ImagePlacementStore>((set) => ({
  session: null,
  open: (session) => set({ session }),
  close: (owner) =>
    set((state) =>
      state.session?.owner === owner ? { session: null } : state,
    ),
}))

/** The placement session for one layer, or `null`. */
export function imagePlacementFor(
  owner: string | null,
): ImagePlacementSession | null {
  if (owner === null) return null
  const session = useImagePlacement.getState().session
  return session?.owner === owner ? session : null
}
