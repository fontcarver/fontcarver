import { create } from 'zustand'

import { call, type Prefs } from '../api'
import { DEFAULT_SNAP_SETTINGS, type SnapSettings } from '../editor/snapping'

/**
 * The kernel's persisted preferences.
 *
 * The editor needs three of them — the grid step, which snap families are live
 * and the arrow-key steps — and the kernel publishes no prefs-changed event, so
 * they are fetched once and refreshed whenever a glyph tab mounts. A failure is
 * not fatal: the defaults below are the ones the kernel itself starts from.
 */
export interface PrefsState {
  prefs: Prefs | null
  /** Whether a load has been attempted, successfully or not. */
  loaded: boolean
  /** Fetches the preferences; safe to call on every mount. */
  load: () => Promise<void>
  /** Writes new preferences and keeps the copy in step. */
  save: (prefs: Prefs) => Promise<void>
}

export const usePrefsStore = create<PrefsState>((set, get) => ({
  prefs: null,
  loaded: false,

  load: async () => {
    try {
      const prefs = await call('prefs.get', {})
      set({ prefs, loaded: true })
    } catch {
      // The editor works on the defaults; a toast here would fire on every
      // tab mount for a condition the user cannot act on.
      set({ loaded: true })
    }
  },

  save: async (prefs) => {
    const written = await call('prefs.set', { prefs })
    set({ prefs: written, loaded: true })
    void get
  },
}))

/** The snap settings of the current preferences, or the defaults. */
export function snapSettingsOf(prefs: Prefs | null): SnapSettings {
  if (!prefs) return DEFAULT_SNAP_SETTINGS
  return {
    nodes: prefs.snap.nodes,
    guides: prefs.snap.guides,
    metrics: prefs.snap.metrics,
    grid: prefs.snap.grid,
    components: prefs.snap.components,
    gridSize: Math.max(0, prefs.grid_size),
  }
}

/** The arrow-key steps of the current preferences: plain, Shift, Ctrl. */
export function nudgeStepsOf(prefs: Prefs | null): [number, number, number] {
  return prefs ? prefs.nudge : [1, 10, 100]
}
