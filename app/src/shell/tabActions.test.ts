import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { setTransport } from '../api'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { initialUiState, tabId, useUiStore } from '../stores/uiStore'
import { createMockTransport, makeSummary } from '../test/mockTransport'

import { closeTab, openFontInfoTab, openGlyphTab } from './tabActions'

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
    useSessionStore.setState({ docs: [makeSummary()], activeDoc: 1 })
    useUiStore.getState().openTab({ kind: 'font', doc: 1 })
    openGlyphTab(1, 9)

    closeTab(tabId('font', 1))
    await vi.waitFor(() => expect(useUiStore.getState().tabs).toEqual([]))

    expect(transport.calls.map((call) => call.method)).toContain('session.close')
    expect(useSessionStore.getState().docs).toEqual([])
  })

  it('closes a Glyph tab without touching the document', () => {
    const transport = createMockTransport({})
    setTransport(transport)
    useSessionStore.setState({ docs: [makeSummary()], activeDoc: 1 })
    useUiStore.getState().openTab({ kind: 'font', doc: 1 })
    const glyph = openGlyphTab(1, 9)

    closeTab(glyph)

    expect(transport.calls).toHaveLength(0)
    expect(useUiStore.getState().tabs.map((tab) => tab.kind)).toEqual(['font'])
  })

  it('focuses an existing Font Info tab instead of opening a second one', () => {
    const first = openFontInfoTab(1)
    const second = openFontInfoTab(1)

    expect(second).toBe(first)
    expect(useUiStore.getState().tabs).toHaveLength(1)
  })
})
