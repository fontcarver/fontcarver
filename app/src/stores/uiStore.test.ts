import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import {
  PANEL_BOUNDS,
  applyTheme,
  initialUiState,
  resolveTheme,
  selectActiveDoc,
  selectActiveTab,
  systemTheme,
  tabId,
  useUiStore,
  UNDO_FLASH_MS,
} from './uiStore'

interface FakeMediaQuery {
  matches: boolean
  addEventListener: (type: string, listener: () => void) => void
  removeEventListener: (type: string, listener: () => void) => void
  fire: () => void
}

function stubMatchMedia(matches: boolean): FakeMediaQuery {
  const listeners = new Set<() => void>()
  const query: FakeMediaQuery = {
    matches,
    addEventListener: (_type, listener) => listeners.add(listener),
    removeEventListener: (_type, listener) => listeners.delete(listener),
    fire: () => {
      for (const listener of [...listeners]) listener()
    },
  }
  vi.stubGlobal('matchMedia', () => query)
  return query
}

beforeEach(() => {
  localStorage.clear()
  document.documentElement.removeAttribute('data-theme')
  useUiStore.setState(initialUiState())
})

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('uiStore theme', () => {
  it('writes the resolved theme to the document element', () => {
    useUiStore.getState().setTheme('dark')
    expect(document.documentElement.dataset.theme).toBe('dark')

    useUiStore.getState().setTheme('light')
    expect(document.documentElement.dataset.theme).toBe('light')
  })

  it('persists the preference across store re-creation', () => {
    useUiStore.getState().setTheme('dark')
    useUiStore.setState(initialUiState())

    expect(useUiStore.getState().theme).toBe('dark')
  })

  it('resolves the system preference from prefers-color-scheme', () => {
    stubMatchMedia(true)
    expect(systemTheme()).toBe('dark')
    expect(resolveTheme('system')).toBe('dark')
    expect(resolveTheme('light')).toBe('light')

    useUiStore.getState().setTheme('system')
    expect(document.documentElement.dataset.theme).toBe('dark')
  })

  it('follows a system theme change while the preference is system', () => {
    const query = stubMatchMedia(false)
    useUiStore.getState().setTheme('system')
    expect(document.documentElement.dataset.theme).toBe('light')

    query.matches = true
    applyTheme('system')

    expect(document.documentElement.dataset.theme).toBe('dark')
  })

  it('falls back to light when the platform has no matchMedia', () => {
    vi.stubGlobal('matchMedia', undefined)

    expect(systemTheme()).toBe('light')
  })
})

describe('uiStore panels', () => {
  it('toggles visibility', () => {
    expect(useUiStore.getState().panels.sidebar).toBe(true)
    useUiStore.getState().togglePanel('sidebar')
    expect(useUiStore.getState().panels.sidebar).toBe(false)
  })

  it('clamps sizes to the panel bounds', () => {
    useUiStore.getState().setPanelSize('sidebar', 10)
    expect(useUiStore.getState().sizes.sidebar).toBe(PANEL_BOUNDS.sidebar.min)

    useUiStore.getState().setPanelSize('inspector', 9999)
    expect(useUiStore.getState().sizes.inspector).toBe(
      PANEL_BOUNDS.inspector.max,
    )
  })
})

describe('uiStore tabs', () => {
  it('derives stable ids and never duplicates a tab', () => {
    const first = useUiStore.getState().openTab({ kind: 'font', doc: 1 })
    const again = useUiStore.getState().openTab({ kind: 'font', doc: 1 })

    expect(first).toBe(tabId('font', 1))
    expect(again).toBe(first)
    expect(useUiStore.getState().tabs).toHaveLength(1)
  })

  it('activates a neighbour when the active tab closes', () => {
    const ui = useUiStore.getState()
    ui.openTab({ kind: 'font', doc: 1 })
    const glyph = ui.openTab({ kind: 'glyph', doc: 1, gid: 42 })

    expect(useUiStore.getState().activeTab).toBe(glyph)
    useUiStore.getState().closeTab(glyph)

    expect(useUiStore.getState().activeTab).toBe(tabId('font', 1))
    expect(selectActiveTab(useUiStore.getState())?.kind).toBe('font')
  })

  it('closes every tab of a document at once', () => {
    const ui = useUiStore.getState()
    ui.openTab({ kind: 'font', doc: 1 })
    ui.openTab({ kind: 'glyph', doc: 1, gid: 7 })
    ui.openTab({ kind: 'font', doc: 2 })

    useUiStore.getState().closeTabsForDoc(1)

    expect(useUiStore.getState().tabs.map((tab) => tab.doc)).toEqual([2])
    expect(useUiStore.getState().activeTab).toBe(tabId('font', 2))
  })

  it('derives the active document from the active tab', () => {
    expect(selectActiveDoc(useUiStore.getState())).toBeNull()

    useUiStore.getState().openTab({ kind: 'font', doc: 7 })
    expect(selectActiveDoc(useUiStore.getState())).toBe(7)

    const other = useUiStore.getState().openTab({ kind: 'font', doc: 8 })
    expect(selectActiveDoc(useUiStore.getState())).toBe(8)

    useUiStore.getState().closeTab(other)
    expect(selectActiveDoc(useUiStore.getState())).toBe(7)
  })

  it('cycles through tabs in both directions', () => {
    const ui = useUiStore.getState()
    const a = ui.openTab({ kind: 'font', doc: 1 })
    const b = ui.openTab({ kind: 'font', doc: 2 })

    useUiStore.getState().cycleTab(1)
    expect(useUiStore.getState().activeTab).toBe(a)
    useUiStore.getState().cycleTab(-1)
    expect(useUiStore.getState().activeTab).toBe(b)
  })
})

describe('uiStore toasts', () => {
  it('adds and dismisses notifications', () => {
    const id = useUiStore
      .getState()
      .pushToast({ kind: 'error', message: 'boom' })
    expect(useUiStore.getState().toasts).toHaveLength(1)

    useUiStore.getState().dismissToast(id)
    expect(useUiStore.getState().toasts).toHaveLength(0)
  })
})

describe('undo label flash', () => {
  beforeEach(() => vi.useFakeTimers())
  afterEach(() => vi.useRealTimers())

  it('shows the label of the last edit for 1.5 s', () => {
    useUiStore.getState().flashUndoLabel('Move 3 points')
    expect(useUiStore.getState().undoFlash).toBe('Move 3 points')

    vi.advanceTimersByTime(UNDO_FLASH_MS - 1)
    expect(useUiStore.getState().undoFlash).toBe('Move 3 points')
    vi.advanceTimersByTime(2)
    expect(useUiStore.getState().undoFlash).toBeNull()
  })

  it('restarts the window when another edit lands', () => {
    useUiStore.getState().flashUndoLabel('Move 3 points')
    vi.advanceTimersByTime(1_000)
    useUiStore.getState().flashUndoLabel('Nudge 1 point')

    vi.advanceTimersByTime(1_000)
    expect(useUiStore.getState().undoFlash).toBe('Nudge 1 point')
    vi.advanceTimersByTime(600)
    expect(useUiStore.getState().undoFlash).toBeNull()
  })

  it('ignores an empty label', () => {
    useUiStore.getState().flashUndoLabel(null)
    expect(useUiStore.getState().undoFlash).toBeNull()
  })
})
