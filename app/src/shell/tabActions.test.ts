import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { setTransport } from '../api'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { initialUiState, tabId, useUiStore } from '../stores/uiStore'
import { createMockTransport, makeSummary } from '../test/mockTransport'

import {
  closeTab,
  openFontInfoTab,
  openGlyphTab,
  replaceGlyphTab,
} from './tabActions'

beforeEach(() => {
  localStorage.clear()
  useSessionStore.setState(initialSessionState())
  useUiStore.setState(initialUiState())
})

afterEach(() => {
  setTransport(undefined)
  vi.restoreAllMocks()
})

describe('tabActions', () => {
  it('closes the document when its Font tab is closed', async () => {
    const transport = createMockTransport({ 'session.close': () => null })
    setTransport(transport)
    useSessionStore.setState({ docs: [makeSummary()] })
    useUiStore.getState().openTab({ kind: 'font', doc: 1 })
    openGlyphTab(1, 9)

    closeTab(tabId('font', 1))
    await vi.waitFor(() => expect(useUiStore.getState().tabs).toEqual([]))

    expect(transport.calls.map((call) => call.method)).toContain(
      'session.close',
    )
    expect(useSessionStore.getState().docs).toEqual([])
  })

  it('closes a Glyph tab without touching the document', () => {
    const transport = createMockTransport({})
    setTransport(transport)
    useSessionStore.setState({ docs: [makeSummary()] })
    useUiStore.getState().openTab({ kind: 'font', doc: 1 })
    const glyph = openGlyphTab(1, 9)

    closeTab(glyph)

    expect(transport.calls).toHaveLength(0)
    expect(useUiStore.getState().tabs.map((tab) => tab.kind)).toEqual(['font'])
  })

  it('moves a Glyph tab to another glyph instead of piling tabs up', () => {
    useUiStore.getState().openTab({ kind: 'font', doc: 1 })
    openGlyphTab(1, 9, 'a')

    const next = replaceGlyphTab(1, 9, 10, 'b')

    const tabs = useUiStore.getState().tabs
    expect(tabs.map((tab) => tab.gid)).toEqual([undefined, 10])
    expect(useUiStore.getState().activeTab).toBe(next)
    expect(tabs[1]?.label).toBe('b')
  })

  it('keeps the tab when the step lands on the same glyph', () => {
    openGlyphTab(1, 9, 'a')
    const same = replaceGlyphTab(1, 9, 9)
    expect(same).toBe(tabId('glyph', 1, 9))
    expect(useUiStore.getState().tabs).toHaveLength(1)
  })

  it('keeps a shaped text tab id when its active glyph changes', () => {
    const id = openGlyphTab(1, 9, 'a')
    useUiStore.getState().updateTab(id, {
      textState: {
        text: 'abc',
        activeIndex: 0,
        mode: 'edit',
        master: 'regular',
        size: 120,
        features: [],
        direction: 'ltr',
        previewTheme: 'light',
        previewCollapsed: false,
      },
    })

    expect(replaceGlyphTab(1, 9, 10, 'b')).toBe(id)
    expect(useUiStore.getState().tabs[0]).toMatchObject({ id, gid: 10 })
  })

  it('focuses an existing Font Info tab instead of opening a second one', () => {
    const first = openFontInfoTab(1)
    const second = openFontInfoTab(1)

    expect(second).toBe(first)
    expect(useUiStore.getState().tabs).toHaveLength(1)
  })
})
