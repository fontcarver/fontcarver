import { beforeEach, describe, expect, it } from 'vitest'

import {
  PANEL_BOUNDS,
  initialUiState,
  selectActiveTab,
  tabId,
  useUiStore,
} from './uiStore'

beforeEach(() => {
  localStorage.clear()
  document.documentElement.removeAttribute('data-theme')
  useUiStore.setState(initialUiState())
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
