import { create } from 'zustand'
import {
  parseKeymapFile,
  serializeKeymap,
  type KeymapOverrides,
} from '../shell/keymapDocument'

/** Storage shared by browser reloads and windows of this app origin. */
export const KEYMAP_STORAGE_KEY = 'fontcarver.keymap.v1'

/** Corrupt or unsupported data never disables the shipped shortcuts. */
export function readKeymap(): KeymapOverrides {
  return readKeymapFile().bindings
}

/** The stored keymap, including the conflicts the user deliberately accepted. */
export function readKeymapFile(): {
  bindings: KeymapOverrides
  allowConflicts: readonly string[]
} {
  try {
    const text = globalThis.localStorage?.getItem(KEYMAP_STORAGE_KEY)
    if (!text) return { bindings: {}, allowConflicts: [] }
    const parsed = parseKeymapFile(text)
    return { bindings: parsed.bindings, allowConflicts: parsed.allowConflicts }
  } catch {
    return { bindings: {}, allowConflicts: [] }
  }
}

/** Session dialog visibility and the last successfully persisted keymap. */
export interface KeymapState {
  overrides: KeymapOverrides
  /**
   * Canonical chords whose conflict the user accepted. Dispatch is unchanged —
   * the first available command still wins — but the editor stops blocking on
   * them, so an accepted conflict survives a reload instead of being re-asked.
   */
  allowConflicts: readonly string[]
  open: boolean
  setOpen: (open: boolean) => void
  save: (
    overrides: KeymapOverrides,
    allowConflicts?: readonly string[],
  ) => boolean
}

/** Changes are committed atomically after validation and successful storage. */
export const useKeymapStore = create<KeymapState>((set) => ({
  overrides: readKeymapFile().bindings,
  allowConflicts: readKeymapFile().allowConflicts,
  open: false,
  setOpen: (open) => set({ open }),
  save: (overrides, allowConflicts = []) => {
    try {
      const normalized = parseKeymapFile(
        serializeKeymap(overrides, allowConflicts),
      )
      if (!globalThis.localStorage) return false
      globalThis.localStorage.setItem(
        KEYMAP_STORAGE_KEY,
        serializeKeymap(normalized.bindings, normalized.allowConflicts),
      )
      set({
        overrides: normalized.bindings,
        allowConflicts: normalized.allowConflicts,
      })
      return true
    } catch {
      return false
    }
  },
}))
