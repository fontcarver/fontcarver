import { create } from 'zustand'

import type { DocId, GlyphId, MasterId, RecoveryEntry } from '../api'
import {
  DEFAULT_LANGUAGE,
  i18n,
  normaliseLanguage,
  type Language,
} from '../i18n'
import { errorNotice } from '../i18n/errors'

/** User-selectable colour theme; `system` follows `prefers-color-scheme`. */
export type ThemePreference = 'light' | 'dark' | 'system'

/** Concrete theme written to `data-theme` on the document element. */
export type ResolvedTheme = 'light' | 'dark'

/** Kind of workspace tab. */
export type TabKind = 'font' | 'glyph' | 'fontinfo'

/** Per-tab state for the shaped text editor hosted by a Glyph tab. */
export interface TextTabState {
  readonly text: string
  readonly activeIndex: number | null
  readonly mode: 'text' | 'edit'
  readonly master: MasterId | null
  readonly size: number
  readonly features: Array<[string, boolean]>
  readonly direction: 'ltr' | 'rtl'
  readonly previewTheme: 'light' | 'dark'
  readonly previewCollapsed: boolean
}

/** One workspace tab. Tab ids are derived so re-opening focuses the same tab. */
export interface Tab {
  readonly id: string
  readonly kind: TabKind
  readonly doc: DocId
  readonly gid?: GlyphId
  /** Present once a legacy single-glyph tab has entered the Text workflow. */
  readonly textState?: TextTabState
  /** Optional display label; falls back to the tab kind and document name. */
  readonly label?: string
}

/** A transient, non-blocking notification. */
export interface Toast {
  readonly id: string
  readonly kind: 'info' | 'error'
  readonly message: string
  /** Raw kernel wording, shown behind a "Details" disclosure. */
  readonly details?: string
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

/**
 * Which modal dialog is open, together with what it needs to render.
 *
 * One slot holds at most one dialog: the shell is modal, so opening a second
 * dialog replaces the first rather than stacking two focus traps.
 */
export type DialogState =
  | { readonly kind: 'newFont' }
  | { readonly kind: 'addGlyphs'; readonly doc: DocId }
  | { readonly kind: 'export'; readonly doc: DocId }
  | { readonly kind: 'preferences' }
  | { readonly kind: 'recovery'; readonly entries: readonly RecoveryEntry[] }
  | { readonly kind: 'simplify' }
  | { readonly kind: 'expandStroke' }

/** Inclusive pixel bounds of the resizable panels. */
export const PANEL_BOUNDS: Record<PanelId, { min: number; max: number }> = {
  sidebar: { min: 200, max: 400 },
  inspector: { min: 240, max: 420 },
}

/** How long the status bar shows the last undo label (design spec §10). */
export const UNDO_FLASH_MS = 1500

const STORAGE_KEY = 'fontcutter.ui'

interface PersistedUi {
  theme: ThemePreference
  language: Language
  panels: { sidebar: boolean; inspector: boolean }
  sizes: { sidebar: number; inspector: number }
  /**
   * Open/closed state of the Edit tab's inspector accordion, keyed by section
   * id (design spec §7: "each remembers open/closed state"). A section with no
   * entry uses its own default, so a new panel can ship open without having to
   * migrate everyone's stored record.
   */
  sections: Record<string, boolean>
  textPreviewVisible: boolean
}

const DEFAULT_PERSISTED: PersistedUi = {
  theme: 'system',
  language: DEFAULT_LANGUAGE,
  panels: { sidebar: true, inspector: true },
  sizes: { sidebar: 240, inspector: 300 },
  sections: {},
  textPreviewVisible: true,
}

/** Keeps only the boolean entries of a stored section record. */
function readSections(value: unknown): Record<string, boolean> {
  if (typeof value !== 'object' || value === null) return {}
  const sections: Record<string, boolean> = {}
  for (const [key, open] of Object.entries(value))
    if (typeof open === 'boolean') sections[key] = open
  return sections
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
      sections: readSections(parsed.sections),
      textPreviewVisible:
        parsed.textPreviewVisible ?? DEFAULT_PERSISTED.textPreviewVisible,
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
  dialog: DialogState | null
  /**
   * How many modal dialogs are on screen.
   *
   * The shared `Dialog` primitive counts itself in and out, so the global
   * shortcut dispatcher can leave the keyboard to the dialog instead of, say,
   * closing the document behind it.
   */
  modalCount: number
  /** Documents whose "recovered from…" banner the user has dismissed. */
  dismissedRecoveries: readonly DocId[]
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
  setSection: (id: string, open: boolean) => void
  toggleSection: (id: string, fallback: boolean) => void
  openTab: (tab: Omit<Tab, 'id'>) => string
  setTabLabel: (id: string, label: string) => void
  updateTab: (
    id: string,
    patch: Partial<Omit<Tab, 'id' | 'kind' | 'doc'>>,
  ) => void
  setActiveTab: (id: string) => void
  closeTab: (id: string) => void
  closeTabsForDoc: (doc: DocId) => void
  cycleTab: (delta: number) => void
  setPaletteOpen: (open: boolean) => void
  setPathPrompt: (prompt: PathPrompt | null) => void
  setClosePrompt: (prompt: ClosePrompt | null) => void
  setDialog: (dialog: DialogState | null) => void
  openModal: () => void
  closeModal: () => void
  dismissRecoveryBanner: (doc: DocId) => void
  pushToast: (toast: Omit<Toast, 'id'>) => string
  dismissToast: (id: string) => void
  flashUndoLabel: (label: string | null) => void
  setTextPreviewVisible: (visible: boolean) => void
  toggleTextPreview: () => void
}

function persistedOf(state: UiState): PersistedUi {
  return {
    theme: state.theme,
    language: state.language,
    panels: state.panels,
    sizes: state.sizes,
    sections: state.sections,
    textPreviewVisible: state.textPreviewVisible,
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
  | 'setSection'
  | 'toggleSection'
  | 'openTab'
  | 'setTabLabel'
  | 'updateTab'
  | 'setActiveTab'
  | 'closeTab'
  | 'closeTabsForDoc'
  | 'cycleTab'
  | 'setPaletteOpen'
  | 'setPathPrompt'
  | 'setClosePrompt'
  | 'setDialog'
  | 'openModal'
  | 'closeModal'
  | 'dismissRecoveryBanner'
  | 'pushToast'
  | 'dismissToast'
  | 'flashUndoLabel'
  | 'setTextPreviewVisible'
  | 'toggleTextPreview'
> {
  return {
    ...readPersisted(),
    tabs: [],
    activeTab: null,
    paletteOpen: false,
    pathPrompt: null,
    closePrompt: null,
    dialog: null,
    modalCount: 0,
    dismissedRecoveries: [],
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

  setSection: (id, open) => {
    set((state) => ({ sections: { ...state.sections, [id]: open } }))
    writePersisted(persistedOf(get()))
  },

  toggleSection: (id, fallback) => {
    get().setSection(id, !(get().sections[id] ?? fallback))
  },

  setPanelSize: (panel, size) => {
    const { min, max } = PANEL_BOUNDS[panel]
    set((state) => ({
      sizes: { ...state.sizes, [panel]: clamp(size, min, max) },
    }))
    writePersisted(persistedOf(get()))
  },

  openTab: (tab) => {
    const baseId = tabId(tab.kind, tab.doc, tab.gid)
    const occupied = get().tabs.find((candidate) => candidate.id === baseId)
    let id = baseId
    if (
      tab.kind === 'glyph' &&
      occupied?.textState &&
      occupied.gid !== tab.gid
    ) {
      let suffix = 2
      while (
        get().tabs.some((candidate) => candidate.id === `${baseId}:${suffix}`)
      )
        suffix += 1
      id = `${baseId}:${suffix}`
    }
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

  updateTab: (id, patch) => {
    set((state) => ({
      tabs: state.tabs.map((tab) =>
        tab.id === id ? { ...tab, ...patch, id: tab.id } : tab,
      ),
    }))
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
      // A closed document takes its dismissed-banner marker with it, so the
      // list cannot grow for the lifetime of the session.
      const dismissedRecoveries = state.dismissedRecoveries.filter(
        (entry) => entry !== doc,
      )
      const tabs = state.tabs.filter((tab) => tab.doc !== doc)
      if (tabs.length === state.tabs.length)
        return dismissedRecoveries.length === state.dismissedRecoveries.length
          ? state
          : { ...state, dismissedRecoveries }
      const activeSurvives = tabs.some((tab) => tab.id === state.activeTab)
      return {
        ...state,
        tabs,
        dismissedRecoveries,
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

  setDialog: (dialog) => set({ dialog }),

  openModal: () => set((state) => ({ modalCount: state.modalCount + 1 })),

  closeModal: () =>
    set((state) => ({ modalCount: Math.max(0, state.modalCount - 1) })),

  dismissRecoveryBanner: (doc) =>
    set((state) =>
      state.dismissedRecoveries.includes(doc)
        ? state
        : { dismissedRecoveries: [...state.dismissedRecoveries, doc] },
    ),

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

  setTextPreviewVisible: (textPreviewVisible) => {
    set({ textPreviewVisible })
    writePersisted(persistedOf(get()))
  },

  toggleTextPreview: () =>
    get().setTextPreviewVisible(!get().textPreviewVisible),
}))

/**
 * Reports a rejected kernel call as an error toast.
 *
 * The single place the interface turns a thrown value into a notification, so
 * every failure gets the same treatment: a translated summary for the error
 * code and the kernel's own wording under "Details".
 */
export function reportError(error: unknown): void {
  const notice = errorNotice(error, (key, options) => i18n.t(key, options))
  useUiStore.getState().pushToast({
    kind: 'error',
    message: notice.message,
    ...(notice.details === null ? {} : { details: notice.details }),
  })
}

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
