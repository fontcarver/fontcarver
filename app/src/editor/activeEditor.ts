/**
 * The mounted editor, reachable from the command registry.
 *
 * Commands (`Delete`, `Mod+A`, the smooth toggle, the Info box shortcut) have
 * to reach the editor of the active tab, which is not something a store can
 * hold: the controller owns a live gesture and a mutable viewport. The mounted
 * view registers itself here and clears its own registration on unmount, which
 * is the same pattern the P0 glyph view uses for zoom and glyph stepping.
 */

import type { EditorController } from './editorController'

let current: EditorController | null = null

declare global {
  /** Dev-only handle used by interactive verification sessions. */
  var __fwEditor: EditorController | null | undefined
}

/** Registers the mounted editor; the disposer only clears its own handle. */
export function registerEditor(controller: EditorController): () => void {
  current = controller
  if (import.meta.env.DEV) globalThis.__fwEditor = controller
  return () => {
    if (current !== controller) return
    current = null
    if (import.meta.env.DEV) globalThis.__fwEditor = null
  }
}

/** The mounted editor, or `null` when no glyph tab is showing. */
export function activeEditor(): EditorController | null {
  return current
}
