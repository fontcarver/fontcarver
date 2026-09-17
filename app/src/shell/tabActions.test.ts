import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import * as editors from '../editor/activeEditor'
import type { EditorController } from '../editor/editorController'
import { setTransport } from '../api'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { initialUiState, tabId, useUiStore } from '../stores/uiStore'
import { createMockTransport, makeSummary } from '../test/mockTransport'

import {
  closeTab,
  changeGlyphTab,
  openFontInfoTab,
  openGlyphTab as openGlyphTabAction,
  replaceGlyphTab,
} from './tabActions'

async function openGlyphTab(...args: Parameters<typeof openGlyphTabAction>) {
  const id = await openGlyphTabAction(...args)
  if (id === undefined) throw new Error('Expected the glyph tab to open')
  return id
}

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
    const transport = createMockTransport({
      'doc.summary': () => makeSummary({ dirty: false }),
      'session.close': () => null,
    })
    setTransport(transport)
    useSessionStore.setState({ docs: [makeSummary()] })
    useUiStore.getState().openTab({ kind: 'font', doc: 1 })
    await openGlyphTab(1, 9)

    await closeTab(tabId('font', 1))
    await vi.waitFor(() => expect(useUiStore.getState().tabs).toEqual([]))

    expect(transport.calls.map((call) => call.method)).toContain(
      'session.close',
    )
    expect(useSessionStore.getState().docs).toEqual([])
  })

  it('closes a Glyph tab without touching the document', async () => {
    const transport = createMockTransport({})
    setTransport(transport)
    useSessionStore.setState({ docs: [makeSummary()] })
    useUiStore.getState().openTab({ kind: 'font', doc: 1 })
    const glyph = await openGlyphTab(1, 9)

    await closeTab(glyph)

    expect(transport.calls).toHaveLength(0)
    expect(useUiStore.getState().tabs.map((tab) => tab.kind)).toEqual(['font'])
  })

  it('moves a Glyph tab to another glyph instead of piling tabs up', async () => {
    useUiStore.getState().openTab({ kind: 'font', doc: 1 })
    await openGlyphTab(1, 9, 'a')

    const next = replaceGlyphTab(1, 9, 10, 'b')

    const tabs = useUiStore.getState().tabs
    expect(tabs.map((tab) => tab.gid)).toEqual([undefined, 10])
    expect(useUiStore.getState().activeTab).toBe(next)
    expect(tabs[1]?.label).toBe('b')
  })

  it('keeps the tab when the step lands on the same glyph', async () => {
    await openGlyphTab(1, 9, 'a')
    const same = replaceGlyphTab(1, 9, 9)
    expect(same).toBe(tabId('glyph', 1, 9))
    expect(useUiStore.getState().tabs).toHaveLength(1)
  })

  it('keeps a shaped text tab id when its active glyph changes', async () => {
    const id = await openGlyphTab(1, 9, 'a')
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
    expect(useUiStore.getState().tabs[0]).toMatchObject({
      id,
      gid: 10,
      textState: { activeIndex: null },
    })
  })

  it('steps the active text tab when two tabs share the same active glyph', async () => {
    const first = await openGlyphTab(1, 9, 'a')
    const textState = {
      text: 'abc',
      activeIndex: 0,
      mode: 'edit' as const,
      master: 'regular',
      size: 120,
      features: [] as Array<[string, boolean]>,
      direction: 'ltr' as const,
      previewTheme: 'light' as const,
      previewCollapsed: false,
    }
    useUiStore.getState().updateTab(first, { textState })
    const second = useUiStore
      .getState()
      .openTab({ kind: 'glyph', doc: 1, gid: 10, label: 'b' })
    useUiStore.getState().updateTab(second, { gid: 9, textState })
    useUiStore.getState().setActiveTab(second)

    expect(replaceGlyphTab(1, 9, 11, 'c')).toBe(second)
    expect(
      useUiStore.getState().tabs.find((tab) => tab.id === first)?.gid,
    ).toBe(9)
    expect(
      useUiStore.getState().tabs.find((tab) => tab.id === second)?.gid,
    ).toBe(11)
  })

  it('opens a collision-free tab when an origin id holds other typed text', async () => {
    const original = await openGlyphTab(1, 9, 'a')
    useUiStore.getState().updateTab(original, {
      gid: 10,
      textState: {
        text: 'typed text',
        activeIndex: 1,
        mode: 'edit',
        master: 'regular',
        size: 120,
        features: [],
        direction: 'ltr',
        previewTheme: 'light',
        previewCollapsed: false,
      },
    })

    const reopened = await openGlyphTab(1, 9, 'a')

    expect(reopened).not.toBe(original)
    expect(useUiStore.getState().tabs).toHaveLength(2)
    expect(useUiStore.getState().tabs[0]?.textState?.text).toBe('typed text')
    expect(useUiStore.getState().tabs[1]).toMatchObject({ gid: 9, label: 'a' })
  })

  it('focuses a text tab already targeting the requested glyph', async () => {
    const first = await openGlyphTab(1, 9, 'a')
    useUiStore.getState().updateTab(first, {
      gid: 10,
      textState: {
        text: 'abc',
        activeIndex: 1,
        mode: 'edit',
        master: 'regular',
        size: 120,
        features: [],
        direction: 'ltr',
        previewTheme: 'light',
        previewCollapsed: false,
      },
    })

    expect(await openGlyphTab(1, 10, 'b')).toBe(first)
    expect(useUiStore.getState().tabs).toHaveLength(1)
  })

  it('steps a suffixed legacy tab without closing the typed origin tab', async () => {
    const original = await openGlyphTab(1, 9, 'a')
    useUiStore.getState().updateTab(original, {
      gid: 10,
      textState: {
        text: 'typed text',
        activeIndex: 1,
        mode: 'edit',
        master: 'regular',
        size: 120,
        features: [],
        direction: 'ltr',
        previewTheme: 'light',
        previewCollapsed: false,
      },
    })
    const suffixed = await openGlyphTab(1, 9, 'a')

    const next = replaceGlyphTab(1, 9, 11, 'c')

    expect(next).not.toBe(suffixed)
    expect(
      useUiStore.getState().tabs.find((tab) => tab.id === original)?.textState
        ?.text,
    ).toBe('typed text')
    expect(useUiStore.getState().tabs.some((tab) => tab.id === suffixed)).toBe(
      false,
    )
  })

  it('does not flush the new editor when a delayed old glyph step arrives', async () => {
    const original = await openGlyphTab(1, 9, 'a')
    const next = await openGlyphTab(1, 20, 'other')
    useUiStore.getState().updateTab(next, { gid: 9 })
    const prepareTransition = vi.fn().mockResolvedValue(undefined)
    vi.spyOn(editors, 'activeEditor').mockReturnValue({
      prepareTransition,
    } as unknown as EditorController)
    await changeGlyphTab(1, 9, 10, 'b', original)
    expect(prepareTransition).not.toHaveBeenCalled()
    expect(useUiStore.getState().activeTab).toBe(next)
  })

  it('focuses an existing Font Info tab instead of opening a second one', async () => {
    const first = await openFontInfoTab(1)
    const second = await openFontInfoTab(1)

    expect(second).toBe(first)
    expect(useUiStore.getState().tabs).toHaveLength(1)
  })
})
