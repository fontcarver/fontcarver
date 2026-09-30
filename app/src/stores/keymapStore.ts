import { create } from 'zustand'
import {
  parseKeymapDocument,
  serializeKeymap,
  type KeymapOverrides,
} from '../shell/keymapDocument'

/** Storage shared by browser reloads and windows of this app origin. */
export const KEYMAP_STORAGE_KEY = 'fontcarver.keymap.v1'

/** Corrupt or unsupported data never disables the shipped shortcuts. */
export function readKeymap(): KeymapOverrides {
  try {
    const text = globalThis.localStorage?.getItem(KEYMAP_STORAGE_KEY)
    return text ? parseKeymapDocument(text) : {}
  } catch {
    return {}
  }
}

/** Session dialog visibility and the last successfully persisted keymap. */
export interface KeymapState {
  overrides: KeymapOverrides
  open: boolean
  setOpen: (open: boolean) => void
  save: (overrides: KeymapOverrides) => boolean
}

/** Changes are committed atomically after validation and successful storage. */
export const useKeymapStore = create<KeymapState>((set) => ({
  overrides: readKeymap(),
  open: false,
  setOpen: (open) => set({ open }),
  save: (overrides) => {
    try {
      const normalized = parseKeymapDocument(serializeKeymap(overrides))
      if (!globalThis.localStorage) return false
      globalThis.localStorage.setItem(
        KEYMAP_STORAGE_KEY,
        serializeKeymap(normalized),
      )
      set({ overrides: normalized })
      return true
    } catch {
      return false
    }
  },
}))
