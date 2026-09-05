import { create } from 'zustand'

import { call, type Prefs } from '../api'
import { normaliseLanguage } from '../i18n'

import { errorMessage } from './sessionStore'
import { useUiStore, type ThemePreference } from './uiStore'

/**
 * The preference record the kernel falls back to, mirrored here so "Reset to
 * defaults" does not need a round trip and so a kernel that has never written a
 * `prefs.json` and this store agree on what "default" means.
 */
export const DEFAULT_PREFS: Prefs = Object.freeze<Prefs>({
  theme: 'system',
  language: 'en',
  recent_limit: 20,
  autosave_seconds: 60,
  grid_size: 1,
  snap: {
    nodes: true,
    guides: true,
    metrics: true,
    grid: true,
    components: true,
  },
  nudge: [1, 10, 100],
})

/** Narrows a stored theme string to the shell's own preference type. */
function themeOf(value: string): ThemePreference {
  return value === 'light' || value === 'dark' ? value : 'system'
}

/** Mirrors the two preferences the shell also owns into the UI store. */
function applyToShell(prefs: Prefs): void {
  const ui = useUiStore.getState()
  const theme = themeOf(prefs.theme)
  if (ui.theme !== theme) ui.setTheme(theme)
  const language = normaliseLanguage(prefs.language)
  if (ui.language !== language) ui.setLanguage(language)
}

/** Persisted application preferences, shared by every view that reads them. */
export interface PrefsState {
  prefs: Prefs
  /** Whether the kernel's values have been read at least once. */
  loaded: boolean
  load: () => Promise<void>
  /** Applies a patch immediately and persists the complete record. */
  update: (patch: Partial<Prefs>) => Promise<void>
  reset: () => Promise<void>
}

/** Creates the initial, action-free part of the preference state. */
export function initialPrefsState(): Pick<PrefsState, 'prefs' | 'loaded'> {
  return { prefs: DEFAULT_PREFS, loaded: false }
}

export const usePrefsStore = create<PrefsState>((set, get) => {
  /**
   * Writes run one at a time, in the order they were asked for.
   *
   * `prefs.set` replaces the whole record, so two writes that overlap would
   * each send their own view of it and the last one on the wire would win —
   * silently dropping the other's change.
   */
  let queue: Promise<void> = Promise.resolve()

  /** Changes asked for but not yet written, merged in arrival order. */
  let pending: Partial<Prefs> | undefined

  const enqueue = (task: () => Promise<void>): Promise<void> => {
    // Both handlers, so one failed write cannot stall every later one.
    queue = queue.then(task, task)
    return queue
  }

  const persist = async (prefs: Prefs): Promise<void> => {
    const previous = get().prefs
    // Applying first keeps the dialog responsive; a rejected write rolls back
    // so the interface never claims a preference the kernel did not store.
    set({ prefs })
    applyToShell(prefs)
    try {
      set({ prefs: await call('prefs.set', { prefs }), loaded: true })
    } catch (error) {
      set({ prefs: previous })
      applyToShell(previous)
      useUiStore
        .getState()
        .pushToast({ kind: 'error', message: errorMessage(error) })
    }
  }

  return {
    ...initialPrefsState(),

    load: async () => {
      try {
        const prefs = await call('prefs.get', {})
        set({ prefs, loaded: true })
        applyToShell(prefs)
      } catch (error) {
        useUiStore
          .getState()
          .pushToast({ kind: 'error', message: errorMessage(error) })
      }
    },

    /**
     * Merges a change into the kernel's own record.
     *
     * `prefs.set` replaces the whole record, and before a successful load this
     * store holds nothing but [`DEFAULT_PREFS`] — writing then would silently
     * overwrite the stored autosave interval, recent limit, nudge steps, grid
     * size and snapping with defaults. So an unloaded store reads first, and
     * refuses the write outright when that read fails.
     *
     * Changes that arrive while an earlier one is still in flight — a theme
     * click from the View menu and a dialog field during the same start-up
     * read, say — are merged and written together, so they cost one read and
     * one write and neither can be lost.
     */
    update: (patch) => {
      pending = { ...pending, ...patch }
      return enqueue(async () => {
        const merged = pending
        pending = undefined
        // An earlier task in this queue already wrote these changes.
        if (!merged) return
        if (!get().loaded) {
          await get().load()
          if (!get().loaded) return
        }
        await persist({ ...get().prefs, ...merged })
      })
    },

    // Reset is the one write that means "the defaults", loaded or not, and it
    // supersedes anything still queued.
    reset: () =>
      enqueue(async () => {
        pending = undefined
        await persist({ ...DEFAULT_PREFS })
      }),
  }
})
