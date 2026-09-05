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

/**
 * One write, captured the moment the caller asked for it.
 *
 * An intent says what was meant, not what to send: `prefs.set` replaces the
 * whole record, so the record itself is built when the intent runs, out of
 * whatever the kernel has confirmed by then.
 */
type Intent =
  | { readonly kind: 'update'; readonly patch: Partial<Prefs> }
  | { readonly kind: 'reset' }

/** Persisted application preferences, shared by every view that reads them. */
export interface PrefsState {
  prefs: Prefs
  /** Whether the kernel's values have been read at least once. */
  loaded: boolean
  /**
   * The record the kernel last confirmed, and the base every write is built
   * from; `null` until a read or a write has come back.
   *
   * `prefs` runs ahead of this while a write is on the wire — that is what
   * keeps the dialog responsive — and falls back to it when a write is
   * rejected, so a dropped write leaves neither the interface nor the write
   * after it believing something the kernel never stored.
   */
  confirmed: Prefs | null
  load: () => Promise<void>
  /** Applies a patch immediately and persists the complete record. */
  update: (patch: Partial<Prefs>) => Promise<void>
  reset: () => Promise<void>
}

/** Creates the initial, action-free part of the preference state. */
export function initialPrefsState(): Pick<
  PrefsState,
  'prefs' | 'loaded' | 'confirmed'
> {
  return { prefs: DEFAULT_PREFS, loaded: false, confirmed: null }
}

export const usePrefsStore = create<PrefsState>((set, get) => {
  /**
   * Intents run one at a time, in the order they were asked for.
   *
   * `prefs.set` replaces the whole record, so two writes that overlap would
   * each send their own view of it and the last one on the wire would win,
   * silently dropping the other's change. Nothing is shared between intents and
   * none is folded into another: one call is one write. Preference edits are
   * rare, so an order that can be reasoned about is worth more than the writes
   * a coalescing buffer would save.
   */
  let queue: Promise<void> = Promise.resolve()

  /**
   * The record an intent sends, built when that intent runs.
   *
   * `undefined` means "write nothing": an update needs the kernel's own record
   * to lay its patch over, and before a successful read this store holds
   * nothing but [`DEFAULT_PREFS`] — writing then would silently overwrite the
   * stored autosave interval, recent limit, nudge steps, grid size and snapping
   * with defaults. So an unloaded store reads first and gives up when that read
   * fails. A reset means the defaults and needs no read at all.
   */
  const recordFor = async (intent: Intent): Promise<Prefs | undefined> => {
    if (intent.kind === 'reset') return DEFAULT_PREFS
    if (!get().confirmed) await get().load()
    const confirmed = get().confirmed
    return confirmed ? { ...confirmed, ...intent.patch } : undefined
  }

  /** Writes one intent, reports it, and leaves the queue behind it alone. */
  const run = async (intent: Intent): Promise<void> => {
    const prefs = await recordFor(intent)
    if (!prefs) return
    // Applying first keeps the dialog responsive; a rejected write falls back
    // to what the kernel confirmed, so the interface never claims a preference
    // the kernel did not store and the next intent still builds on the record
    // that is really there.
    const confirmed = get().confirmed ?? get().prefs
    set({ prefs })
    applyToShell(prefs)
    try {
      const saved = await call('prefs.set', { prefs })
      set({ prefs: saved, loaded: true, confirmed: saved })
      applyToShell(saved)
    } catch (error) {
      set({ prefs: confirmed })
      applyToShell(confirmed)
      useUiStore
        .getState()
        .pushToast({ kind: 'error', message: errorMessage(error) })
    }
  }

  const enqueue = (intent: Intent): Promise<void> => {
    // `run` reports a failed write itself, and the chain is caught so that one
    // rejection can neither stall the intents behind it nor surface as an
    // unhandled rejection in a caller that did not await what it was handed.
    queue = queue.then(() => run(intent)).catch(() => undefined)
    return queue
  }

  return {
    ...initialPrefsState(),

    load: async () => {
      try {
        const prefs = await call('prefs.get', {})
        set({ prefs, loaded: true, confirmed: prefs })
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
     * The patch is captured here, at call time, and carried by an intent of its
     * own: one call is one `prefs.set`, and the writes go out in the order the
     * user acted in. Which record the patch lands on is decided when the write
     * goes out — the last one the kernel confirmed — so a change made while an
     * earlier write is still on the wire cannot be lost, a change made after a
     * reset lands on top of the defaults, and a rejected write changes nothing
     * about what the writes after it are built on.
     */
    update: (patch) => enqueue({ kind: 'update', patch: { ...patch } }),

    /**
     * Restores the shipped defaults, loaded or not.
     *
     * It supersedes the changes asked for before it in the only sense a
     * whole-record write can: it is written after them. A field edited after
     * the reset button was pressed is a later intent, so it lands on top of the
     * defaults rather than being swallowed by the reset.
     */
    reset: () => enqueue({ kind: 'reset' }),
  }
})
