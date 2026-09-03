import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { setTransport } from '../api'
import { createMockTransport, makeSummary } from '../test/mockTransport'

import { initialSessionState, useSessionStore } from './sessionStore'
import { initialUiState, tabId, useUiStore } from './uiStore'

beforeEach(() => {
  localStorage.clear()
  useSessionStore.setState(initialSessionState())
  useUiStore.setState(initialUiState())
})

afterEach(() => {
  setTransport(undefined)
  vi.restoreAllMocks()
})

describe('sessionStore.open', () => {
  it('calls the kernel, records the document and opens a Font tab', async () => {
    const summary = makeSummary()
    const transport = createMockTransport({
      'session.open': () => summary,
      'session.recent': () => ['/fonts/DejaVuSans.ttf'],
    })
    setTransport(transport)

    const result = await useSessionStore
      .getState()
      .open('/fonts/DejaVuSans.ttf')

    expect(result).toEqual(summary)
    expect(transport.calls[0]).toEqual({
      method: 'session.open',
      params: { path: '/fonts/DejaVuSans.ttf', face_index: null },
    })
    expect(useSessionStore.getState().docs).toEqual([summary])
    expect(useSessionStore.getState().activeDoc).toBe(1)
    expect(useUiStore.getState().tabs).toEqual([
      { id: tabId('font', 1), kind: 'font', doc: 1 },
    ])
  })

  it('surfaces kernel failures as a toast and keeps the session empty', async () => {
    setTransport(
      createMockTransport({
        'session.open': () => {
          throw new Error('unsupported format')
        },
      }),
    )

    const result = await useSessionStore.getState().open('/fonts/broken.bin')

    expect(result).toBeNull()
    expect(useSessionStore.getState().docs).toEqual([])
    expect(useUiStore.getState().toasts[0]).toMatchObject({
      kind: 'error',
      message: 'unsupported format',
    })
  })

  it('reports open warnings as informational toasts', async () => {
    setTransport(
      createMockTransport({
        'session.open': () => makeSummary({ warnings: ['dropped 2 tables'] }),
        'session.recent': () => [],
      }),
    )

    await useSessionStore.getState().open('/fonts/DejaVuSans.ttf')

    expect(useUiStore.getState().toasts[0]).toMatchObject({
      kind: 'info',
      message: 'dropped 2 tables',
    })
  })
})

describe('sessionStore.save', () => {
  it('prompts for a path when the document is not a native .fwf', async () => {
    const transport = createMockTransport({})
    setTransport(transport)
    useSessionStore.setState({ docs: [makeSummary()], activeDoc: 1 })

    await useSessionStore.getState().save(1)

    expect(transport.calls).toHaveLength(0)
    expect(useUiStore.getState().pathPrompt).toEqual({ mode: 'saveAs', doc: 1 })
  })

  it('saves straight away once the document has a .fwf path', async () => {
    const saved = makeSummary({ path: '/docs/dejavu.fwf', dirty: false })
    const transport = createMockTransport({
      'session.save': () => saved,
      'session.recent': () => [],
    })
    setTransport(transport)
    useSessionStore.setState({
      docs: [makeSummary({ path: '/docs/dejavu.fwf', dirty: true })],
      activeDoc: 1,
    })

    await useSessionStore.getState().save(1)

    expect(transport.calls[0]).toEqual({
      method: 'session.save',
      params: { doc: 1, path: null },
    })
    expect(useSessionStore.getState().docs[0]?.dirty).toBe(false)
  })
})

describe('sessionStore.close', () => {
  it('removes the document and every tab that belonged to it', async () => {
    setTransport(createMockTransport({ 'session.close': () => null }))
    useSessionStore.setState({ docs: [makeSummary()], activeDoc: 1 })
    useUiStore.getState().openTab({ kind: 'font', doc: 1 })
    useUiStore.getState().openTab({ kind: 'glyph', doc: 1, gid: 3 })

    await useSessionStore.getState().close(1)

    expect(useSessionStore.getState().docs).toEqual([])
    expect(useSessionStore.getState().activeDoc).toBeNull()
    expect(useUiStore.getState().tabs).toEqual([])
  })

  it('asks before closing a dirty document and keeps it when declined', async () => {
    const transport = createMockTransport({ 'session.close': () => null })
    setTransport(transport)
    useSessionStore.setState({
      docs: [makeSummary({ dirty: true })],
      activeDoc: 1,
    })
    const confirm = vi.spyOn(globalThis, 'confirm').mockReturnValue(false)

    await useSessionStore.getState().requestClose(1)

    expect(confirm).toHaveBeenCalledOnce()
    expect(transport.calls).toHaveLength(0)
    expect(useSessionStore.getState().docs).toHaveLength(1)
  })
})

describe('sessionStore events', () => {
  it('adopts documents already open in the kernel and follows change events', async () => {
    const transport = createMockTransport({
      'session.list': () => [makeSummary()],
      'session.recent': () => [],
    })
    setTransport(transport)

    const stop = useSessionStore.getState().start()
    await vi.waitFor(() =>
      expect(useSessionStore.getState().docs).toHaveLength(1),
    )
    expect(useUiStore.getState().activeTab).toBe(tabId('font', 1))

    transport.emit({
      doc: 1,
      generation: 2,
      change: {
        glyphs: [4],
        font_info: false,
        masters: false,
        glyph_order: false,
      },
      undo_label: 'Move nodes',
      redo_label: null,
      dirty: true,
    })

    expect(useSessionStore.getState().docs[0]).toMatchObject({
      dirty: true,
      undo_label: 'Move nodes',
    })
    stop()
  })
})
