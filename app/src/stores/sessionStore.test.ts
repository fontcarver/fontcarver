import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { setTransport } from '../api'
import { createMockTransport, makeSummary } from '../test/mockTransport'

import { initialDocumentState, useDocumentStore } from './documentStore'
import { initialSessionState, useSessionStore } from './sessionStore'
import { initialUiState, selectActiveDoc, tabId, useUiStore } from './uiStore'

beforeEach(() => {
  localStorage.clear()
  useSessionStore.setState(initialSessionState())
  useUiStore.setState(initialUiState())
  useDocumentStore.setState(initialDocumentState())
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
    expect(useUiStore.getState().tabs).toEqual([
      { id: tabId('font', 1), kind: 'font', doc: 1 },
    ])
    expect(selectActiveDoc(useUiStore.getState())).toBe(1)
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

  it('renders known warning codes through i18n and falls back to raw messages', async () => {
    setTransport(
      createMockTransport({
        'session.open': () =>
          makeSummary({
            warnings: [
              {
                code: 'cff-hints-not-imported',
                message: 'raw CFF warning',
              },
              { code: 'future-warning', message: 'dropped 2 tables' },
            ],
          }),
        'session.recent': () => [],
      }),
    )

    await useSessionStore.getState().open('/fonts/DejaVuSans.ttf')

    expect(useUiStore.getState().toasts[0]).toMatchObject({
      kind: 'info',
      message:
        'CFF outlines were imported without per-glyph stem and hint-mask data',
    })
    expect(useUiStore.getState().toasts[1]).toMatchObject({
      kind: 'info',
      message: 'dropped 2 tables',
    })
  })
})

describe('sessionStore.save', () => {
  it('prompts for a path when the document is not a native .fcf', async () => {
    const transport = createMockTransport({})
    setTransport(transport)
    useSessionStore.setState({ docs: [makeSummary()] })

    const saved = await useSessionStore.getState().save(1)

    expect(saved).toBe(false)
    expect(transport.calls).toHaveLength(0)
    expect(useUiStore.getState().pathPrompt).toEqual({
      mode: 'saveAs',
      doc: 1,
      closeAfterSave: false,
    })
  })

  it('treats a pre-rename .fwf path as a native save target', async () => {
    const saved = makeSummary({ path: '/docs/legacy.fwf', dirty: false })
    const transport = createMockTransport({
      'session.save': () => saved,
      'session.recent': () => [],
    })
    setTransport(transport)
    useSessionStore.setState({
      docs: [makeSummary({ path: '/docs/legacy.fwf', dirty: true })],
    })

    expect(await useSessionStore.getState().save(1)).toBe(true)
    expect(useUiStore.getState().pathPrompt).toBeNull()
  })

  it('saves straight away once the document has a .fcf path', async () => {
    const saved = makeSummary({ path: '/docs/dejavu.fcf', dirty: false })
    const transport = createMockTransport({
      'session.save': () => saved,
      'session.recent': () => [],
    })
    setTransport(transport)
    useSessionStore.setState({
      docs: [makeSummary({ path: '/docs/dejavu.fcf', dirty: true })],
    })

    expect(await useSessionStore.getState().save(1)).toBe(true)
    expect(transport.calls[0]).toEqual({
      method: 'session.save',
      params: { doc: 1, path: null },
    })
    expect(useSessionStore.getState().docs[0]?.dirty).toBe(false)
  })

  it('reports a failed save and resolves false', async () => {
    setTransport(
      createMockTransport({
        'session.save': () => {
          throw new Error('disk full')
        },
      }),
    )
    useSessionStore.setState({ docs: [makeSummary({ path: '/docs/a.fcf' })] })

    expect(await useSessionStore.getState().save(1)).toBe(false)
    expect(useUiStore.getState().toasts[0]).toMatchObject({
      message: 'disk full',
    })
  })
})

describe('sessionStore.close', () => {
  it('removes the document, its tabs and its view state', async () => {
    setTransport(createMockTransport({ 'session.close': () => null }))
    useSessionStore.setState({ docs: [makeSummary()] })
    useUiStore.getState().openTab({ kind: 'font', doc: 1 })
    useUiStore.getState().openTab({ kind: 'glyph', doc: 1, gid: 3 })
    useDocumentStore.getState().setCellSize(1, 96)

    await useSessionStore.getState().close(1)

    expect(useSessionStore.getState().docs).toEqual([])
    expect(useUiStore.getState().tabs).toEqual([])
    expect(selectActiveDoc(useUiStore.getState())).toBeNull()
    expect(useDocumentStore.getState().byDoc[1]).toBeUndefined()
  })

  it('closes a clean document without asking', async () => {
    const transport = createMockTransport({
      'doc.summary': () => makeSummary({ dirty: false }),
      'session.close': () => null,
    })
    setTransport(transport)
    useSessionStore.setState({ docs: [makeSummary({ dirty: false })] })

    await useSessionStore.getState().requestClose(1)

    expect(useUiStore.getState().closePrompt).toBeNull()
    expect(useSessionStore.getState().docs).toEqual([])
  })
})

describe('sessionStore dirty close', () => {
  it('asks before closing a dirty document instead of discarding it', async () => {
    const transport = createMockTransport({
      'doc.summary': () => makeSummary({ dirty: true }),
      'session.close': () => null,
    })
    setTransport(transport)
    useSessionStore.setState({ docs: [makeSummary({ dirty: true })] })

    await useSessionStore.getState().requestClose(1)

    expect(useUiStore.getState().closePrompt).toEqual({ doc: 1 })
    expect(transport.calls.map((call) => call.method)).toEqual(['doc.summary'])
    expect(useSessionStore.getState().docs).toHaveLength(1)
  })

  it('saves and then closes when the document already has a .fcf path', async () => {
    const transport = createMockTransport({
      'session.save': () => makeSummary({ path: '/docs/a.fcf', dirty: false }),
      'doc.summary': () => makeSummary({ path: '/docs/a.fcf', dirty: false }),
      'session.close': () => null,
      'session.recent': () => [],
    })
    setTransport(transport)
    useSessionStore.setState({
      docs: [makeSummary({ path: '/docs/a.fcf', dirty: true })],
    })

    await useSessionStore.getState().saveAndClose(1)

    const methods = transport.calls.map((call) => call.method)
    expect(methods).toContain('session.save')
    expect(methods.indexOf('session.close')).toBeGreaterThan(
      methods.indexOf('session.save'),
    )
    expect(useSessionStore.getState().docs).toEqual([])
  })

  it('keeps the document open when the save fails', async () => {
    const transport = createMockTransport({
      'session.save': () => {
        throw new Error('read-only volume')
      },
    })
    setTransport(transport)
    useSessionStore.setState({
      docs: [makeSummary({ path: '/docs/a.fcf', dirty: true })],
    })

    await useSessionStore.getState().saveAndClose(1)

    expect(transport.calls.map((call) => call.method)).toEqual(['session.save'])
    expect(useSessionStore.getState().docs).toHaveLength(1)
  })

  it('asks for a path first when the dirty document has no native path', async () => {
    const transport = createMockTransport({})
    setTransport(transport)
    useSessionStore.setState({ docs: [makeSummary({ dirty: true })] })

    await useSessionStore.getState().saveAndClose(1)

    expect(transport.calls).toHaveLength(0)
    expect(useUiStore.getState().pathPrompt).toEqual({
      mode: 'saveAs',
      doc: 1,
      closeAfterSave: true,
    })
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
        layout: false,
        bitmaps: false,
        feature_source: false,
        glyph_order: false,
        metrics: false,
        layers: false,
        guides: false,
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

describe('undo label flash', () => {
  it('flashes the label of an edit the kernel reports', () => {
    const transport = createMockTransport({})
    setTransport(transport)
    useSessionStore.setState({ docs: [makeSummary()] })
    const stop = useSessionStore.getState().start()

    transport.emit({
      doc: 1,
      generation: 2,
      dirty: true,
      undo_label: 'Move 3 points',
      redo_label: null,
      change: {
        glyphs: [1],
        font_info: false,
        masters: false,
        layout: false,
        bitmaps: false,
        feature_source: false,
        glyph_order: false,
        metrics: false,
        layers: false,
        guides: false,
      },
    })

    expect(useUiStore.getState().undoFlash).toBe('Move 3 points')
    stop()
  })

  it('says nothing when an event only replays the label it already had', () => {
    const transport = createMockTransport({})
    setTransport(transport)
    useSessionStore.setState({
      docs: [makeSummary({ undo_label: 'Move 3 points' })],
    })
    const stop = useSessionStore.getState().start()

    transport.emit({
      doc: 1,
      generation: 3,
      dirty: true,
      undo_label: 'Move 3 points',
      redo_label: null,
      change: {
        glyphs: [1],
        font_info: false,
        masters: false,
        layout: false,
        bitmaps: false,
        feature_source: false,
        glyph_order: false,
        metrics: false,
        layers: false,
        guides: false,
      },
    })

    expect(useUiStore.getState().undoFlash).toBeNull()
    stop()
  })
})
