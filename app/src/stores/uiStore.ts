import { create } from 'zustand'

import type { DocId, GlyphId } from '../api'
import { DEFAULT_LANGUAGE, normaliseLanguage, type Language } from '../i18n'

/** User-selectable colour theme; `system` follows `prefers-color-scheme`. */
export type ThemePreference = 'light' | 'dark' | 'system'

/** Concrete theme written to `data-theme` on the document element. */
export type ResolvedTheme = 'light' | 'dark'

/** Kind of workspace tab. */
export type TabKind = 'font' | 'glyph' | 'fontinfo'

/** One workspace tab. Tab ids are derived so re-opening focuses the same tab. */
export interface Tab {
  readonly id: string
  readonly kind: TabKind
  readonly doc: DocId
  readonly gid?: GlyphId
  /** Optional display label; falls back to the tab kind and document name. */
  readonly label?: string
}

/** A transient, non-blocking notification. */
export interface Toast {
  readonly id: string
  readonly kind: 'info' | 'error'
  readonly message: string
}

/** Which resizable side panel a size or visibility change refers to. */
export type PanelId = 'sidebar' | 'inspector'

/** A path prompt shown when no native file dialog is available. */
export interface PathPrompt {
  readonly mode: 'open' | 'saveAs'
  readonly doc?: DocId
  /** Close the document once the save succeeds (the dirty-close flow). */
  readonly closeAfterSave?: boolean
}

/** Pending "save before closing?" question for one dirty document. */
export interface ClosePrompt {
  readonly doc: DocId
}

/** Inclusive pixel bounds of the resizable panels. */
export const PANEL_BOUNDS: Record<PanelId, { min: number; max: number }> = {
  sidebar: { min: 200, max: 400 },
  inspector: { min: 240, max: 420 },
}

/** How long the status bar shows the last undo label (design spec §10). */
export const UNDO_FLASH_MS = 1500

const STORAGE_KEY = 'fontworkshop.ui'

interface PersistedUi {
  theme: ThemePreference
  language: Language
  panels: { sidebar: boolean; inspector: boolean }
  sizes: { sidebar: number; inspector: number }
}

const DEFAULT_PERSISTED: PersistedUi = {
  theme: 'system',
  language: DEFAULT_LANGUAGE,
  panels: { sidebar: true, inspector: true },
  sizes: { sidebar: 240, inspector: 300 },
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, Math.round(value)))
}

function readPersisted(): PersistedUi {
  try {
    const raw = globalThis.localStorage?.getItem(STORAGE_KEY)
    if (!raw) return DEFAULT_PERSISTED
    const parsed = JSON.parse(raw) as Partial<PersistedUi>
    const theme = parsed.theme
    return {
      theme:
        theme === 'light' || theme === 'dark' || theme === 'system'
          ? theme
          : DEFAULT_PERSISTED.theme,
      language: normaliseLanguage(parsed.language),
      panels: {
        sidebar: parsed.panels?.sidebar ?? DEFAULT_PERSISTED.panels.sidebar,
        inspector:
          parsed.panels?.inspector ?? DEFAULT_PERSISTED.panels.inspector,
      },
      sizes: {
        sidebar: clamp(
          parsed.sizes?.sidebar ?? DEFAULT_PERSISTED.sizes.sidebar,
          PANEL_BOUNDS.sidebar.min,
          PANEL_BOUNDS.sidebar.max,
        ),
        inspector: clamp(
          parsed.sizes?.inspector ?? DEFAULT_PERSISTED.sizes.inspector,
          PANEL_BOUNDS.inspector.min,
          PANEL_BOUNDS.inspector.max,
        ),
      },
    }
  } catch {
    return DEFAULT_PERSISTED
  }
}

function writePersisted(state: PersistedUi): void {
  try {
    globalThis.localStorage?.setItem(STORAGE_KEY, JSON.stringify(state))
  } catch {
    /* Storage is unavailable (private mode, disabled cookies): stay in memory. */
  }
}

/** Reads the operating-system colour preference, defaulting to light. */
export function systemTheme(): ResolvedTheme {
  const query = globalThis.matchMedia?.('(prefers-color-scheme: dark)')
  return query?.matches ? 'dark' : 'light'
}

/** Resolves a theme preference into the theme actually rendered. */
export function resolveTheme(preference: ThemePreference): ResolvedTheme {
  return preference === 'system' ? systemTheme() : preference
}

/** Writes the resolved theme to `data-theme` on the document element. */
export function applyTheme(preference: ThemePreference): void {
  const root = globalThis.document?.documentElement
  if (!root) return
  root.dataset.theme = resolveTheme(preference)
}

/** Tab id for a document-scoped tab; deterministic so tabs are never duplicated. */
export function tabId(kind: TabKind, doc: DocId, gid?: GlyphId): string {
  return kind === 'glyph' ? `glyph:${doc}:${gid}` : `${kind}:${doc}`
}

let toastCounter = 0

/** Application-shell state: theme, language, panels, tabs and notifications. */
export interface UiState extends PersistedUi {
  tabs: Tab[]
  activeTab: string | null
  paletteOpen: boolean
  pathPrompt: PathPrompt | null
  closePrompt: ClosePrompt | null
  toasts: Toast[]
  /**
   * Label of the edit that just happened, shown in the status bar for a moment
   * (design spec §10) so an action that changed nothing visible still says
   * what it did.
   */
  undoFlash: string | null
  setTheme: (theme: ThemePreference) => void
  setLanguage: (language: Language) => void
  setPanelVisible: (panel: PanelId, visible: boolean) => void
  togglePanel: (panel: PanelId) => void
  setPanelSize: (panel: PanelId, size: number) => void
  openTab: (tab: Omit<Tab, 'id'>) => string
  setTabLabel: (id: string, label: string) => void
  setActiveTab: (id: string) => void
  closeTab: (id: string) => void
  closeTabsForDoc: (doc: DocId) => void
  cycleTab: (delta: number) => void
  setPaletteOpen: (open: boolean) => void
  setPathPrompt: (prompt: PathPrompt | null) => void
  setClosePrompt: (prompt: ClosePrompt | null) => void
  pushToast: (toast: Omit<Toast, 'id'>) => string
  dismissToast: (id: string) => void
  flashUndoLabel: (label: string | null) => void
}

function persistedOf(state: UiState): PersistedUi {
  return {
    theme: state.theme,
    language: state.language,
    panels: state.panels,
    sizes: state.sizes,
  }
}

/** Creates the initial, non-persisted part of the UI state. */
export function initialUiState(): Omit<
  UiState,
  | 'setTheme'
  | 'setLanguage'
  | 'setPanelVisible'
  | 'togglePanel'
  | 'setPanelSize'
  | 'openTab'
  | 'setTabLabel'
  | 'setActiveTab'
  | 'closeTab'
  | 'closeTabsForDoc'
  | 'cycleTab'
  | 'setPaletteOpen'
  | 'setPathPrompt'
  | 'setClosePrompt'
  | 'pushToast'
  | 'dismissToast'
  | 'flashUndoLabel'
> {
  return {
    ...readPersisted(),
    tabs: [],
    activeTab: null,
    paletteOpen: false,
    pathPrompt: null,
    closePrompt: null,
    toasts: [],
    undoFlash: null,
  }
}

let undoFlashTimer: ReturnType<typeof setTimeout> | undefined

export const useUiStore = create<UiState>((set, get) => ({
  ...initialUiState(),

  setTheme: (theme) => {
    set({ theme })
    applyTheme(theme)
    writePersisted(persistedOf(get()))
  },

  setLanguage: (language) => {
    set({ language })
    writePersisted(persistedOf(get()))
  },

  setPanelVisible: (panel, visible) => {
    set((state) => ({ panels: { ...state.panels, [panel]: visible } }))
    writePersisted(persistedOf(get()))
  },

  togglePanel: (panel) => {
    get().setPanelVisible(panel, !get().panels[panel])
  },

  setPanelSize: (panel, size) => {
    const { min, max } = PANEL_BOUNDS[panel]
    set((state) => ({
      sizes: { ...state.sizes, [panel]: clamp(size, min, max) },
    }))
    writePersisted(persistedOf(get()))
  },

  openTab: (tab) => {
    const id = tabId(tab.kind, tab.doc, tab.gid)
    set((state) => ({
      tabs: state.tabs.some((existing) => existing.id === id)
        ? state.tabs
        : [...state.tabs, { ...tab, id }],
      activeTab: id,
    }))
    return id
  },

  setTabLabel: (id, label) => {
    set((state) => {
      const tab = state.tabs.find((candidate) => candidate.id === id)
      if (!tab || tab.label === label) return state
      return {
        ...state,
        tabs: state.tabs.map((candidate) =>
          candidate.id === id ? { ...candidate, label } : candidate,
        ),
      }
    })
  },

  setActiveTab: (id) => {
    if (get().tabs.some((tab) => tab.id === id)) set({ activeTab: id })
  },

  closeTab: (id) => {
    set((state) => {
      const index = state.tabs.findIndex((tab) => tab.id === id)
      if (index < 0) return state
      const tabs = state.tabs.filter((tab) => tab.id !== id)
      if (state.activeTab !== id) return { ...state, tabs }
      const next = tabs[Math.min(index, tabs.length - 1)]
      return { ...state, tabs, activeTab: next?.id ?? null }
    })
  },

  closeTabsForDoc: (doc) => {
    set((state) => {
      const tabs = state.tabs.filter((tab) => tab.doc !== doc)
      if (tabs.length === state.tabs.length) return state
      const activeSurvives = tabs.some((tab) => tab.id === state.activeTab)
      return {
        ...state,
        tabs,
        activeTab: activeSurvives
          ? state.activeTab
          : (tabs[tabs.length - 1]?.id ?? null),
      }
    })
  },

  cycleTab: (delta) => {
    const { tabs, activeTab } = get()
    if (tabs.length === 0) return
    const index = tabs.findIndex((tab) => tab.id === activeTab)
    const next =
      (((index < 0 ? 0 : index + delta) % tabs.length) + tabs.length) %
      tabs.length
    set({ activeTab: tabs[next]?.id ?? null })
  },

  setPaletteOpen: (paletteOpen) => set({ paletteOpen }),

  setPathPrompt: (pathPrompt) => set({ pathPrompt }),

  setClosePrompt: (closePrompt) => set({ closePrompt }),

  pushToast: (toast) => {
    const id = `toast-${++toastCounter}`
    set((state) => ({ toasts: [...state.toasts, { ...toast, id }] }))
    return id
  },

  dismissToast: (id) =>
    set((state) => ({
      toasts: state.toasts.filter((toast) => toast.id !== id),
    })),

  flashUndoLabel: (label) => {
    if (undoFlashTimer !== undefined) clearTimeout(undoFlashTimer)
    if (!label) {
      set({ undoFlash: null })
      return
    }
    set({ undoFlash: label })
    undoFlashTimer = setTimeout(() => {
      undoFlashTimer = undefined
      set({ undoFlash: null })
    }, UNDO_FLASH_MS)
  },
}))

/** Selector: the currently active tab, if any. */
export function selectActiveTab(state: UiState): Tab | null {
  return state.tabs.find((tab) => tab.id === state.activeTab) ?? null
}

/**
 * Selector: the document the active tab belongs to. The active document is
 * always derived from the active tab so no copy of it can go stale.
 */
export function selectActiveDoc(state: UiState): DocId | null {
  return selectActiveTab(state)?.doc ?? null
}

/** The active document id outside React. */
export function activeDocId(): DocId | null {
  return selectActiveDoc(useUiStore.getState())
}
