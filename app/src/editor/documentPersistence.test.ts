import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { renderHook } from '@testing-library/react'
import { setTransport } from '../api'
import { Viewport } from '../canvas/Viewport'
import { makeLayer, makeLayerOutline } from '../test/fontFixtures'
import {
  createMockTransport,
  makeSummary,
  noChange,
} from '../test/mockTransport'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { TextMetricsRunner } from '../views/text/textMetrics'
import { useEditorLabels } from '../views/glyph/useEditorLabels'
import { registerEditor } from './activeEditor'
import { EditorController } from './editorController'
import {
  EditSession,
  holdDocumentEdits,
  immediateScheduler,
} from './editSession'
import { withSettledDocument } from './documentPersistence'

let unregister = () => undefined as void
let controller: EditorController | null = null
const summary = makeSummary({ path: '/tmp/persistence.fcf', dirty: false })

function deferred() {
  let resolve!: () => void
  const promise = new Promise<void>((done) => {
    resolve = done
  })
  return { promise, resolve }
}

function editor(doc: number, settle: () => Promise<void>) {
  const viewport = new Viewport()
  viewport.setSize(1000, 500, 1)
  viewport.scale = 1
  const labels = renderHook(useEditorLabels).result.current
  controller = new EditorController(viewport, labels)
  controller.setSource({
    doc,
    gid: 42,
    glyph: 'a',
    outline: makeLayerOutline(makeLayer({ contours: [] })),
    guides: [],
    metrics: {
      upem: 1000,
      ascender: 800,
      descender: -200,
      xHeight: 500,
      capHeight: 700,
      italicAngle: 0,
      width: 600,
      lsb: 0,
      rsb: 600,
      verticalOrigin: null,
      verticalWidth: null,
    },
    master: 'm1',
    curves: 'cubic',
    componentPoints: [],
  })
  controller.setExternalSettler(settle)
  unregister = registerEditor(controller)
  return controller
}

beforeEach(() => {
  useSessionStore.setState({ ...initialSessionState(), docs: [summary] })
  useUiStore.setState(initialUiState())
})
afterEach(() => {
  unregister()
  controller?.dispose()
  controller = null
  setTransport(undefined)
  vi.restoreAllMocks()
})

it('closes a live metrics run before saving and lets the next run edit normally', async () => {
  let token: number | null = null
  let serial = 0
  let width = 600
  let saved = 0
  const errors: unknown[] = []
  const transport = createMockTransport({
    'doc.edit_begin': () => (token = ++serial),
    'doc.edit_apply': ({ token: supplied, command }) => {
      if (token !== supplied) throw new Error('InvalidEditToken')
      if ('SetMetrics' in command) width = command.SetMetrics.width!
      return noChange([42])
    },
    'doc.edit_end': () => {
      token = null
      return null
    },
    'session.save': () => {
      token = null
      saved = width
      return summary
    },
    'session.recent': () => [],
  })
  setTransport(transport)
  const makeRunner = () =>
    new TextMetricsRunner({
      doc: 1,
      glyph: 'a',
      layer: 'm1',
      start: { width, lsb: 0, rsb: width },
      hasOutline: false,
      label: 'Metrics',
      scheduler: immediateScheduler,
      timeout: 60_000,
      onPreview: () => {},
      onEnd: () => {},
      onError: (error) => errors.push(error),
    })
  const first = makeRunner()
  editor(1, () => first.end())
  first.press('rsb', 1)
  expect(await useSessionStore.getState().save(1)).toBe(true)
  expect(first.active).toBe(false)
  expect(saved).toBe(601)
  const next = makeRunner()
  next.press('rsb', 1)
  await next.end()
  expect(width).toBe(602)
  expect(errors).toEqual([])
  const calls = transport.calls.map((call) => call.method)
  expect(calls.indexOf('doc.edit_end')).toBeLessThan(
    calls.indexOf('session.save'),
  )
})

it('waits for old edits and blocks a new begin until persistence finishes', async () => {
  const old = deferred()
  const saving = deferred()
  void holdDocumentEdits(1, old.promise)
  const operation = vi.fn(() => saving.promise)
  const transport = createMockTransport({
    'doc.edit_begin': () => 9,
    'doc.edit_end': () => null,
  })
  setTransport(transport)
  const persisted = withSettledDocument(1, operation)
  const next = new EditSession(1, 'Next edit')
  await Promise.resolve()
  expect(operation).not.toHaveBeenCalled()
  expect(transport.calls).toHaveLength(0)
  old.resolve()
  await vi.waitFor(() => expect(operation).toHaveBeenCalledOnce())
  expect(transport.calls).toHaveLength(0)
  saving.resolve()
  await persisted
  await next.end()
  expect(transport.calls.map((call) => call.method)).toEqual([
    'doc.edit_begin',
    'doc.edit_end',
  ])
})

it('asks about newly dirty work after a delayed close handover', async () => {
  const old = deferred()
  void holdDocumentEdits(1, old.promise)
  const transport = createMockTransport({
    'doc.summary': () => ({ ...summary, dirty: true }),
    'session.close': () => null,
  })
  setTransport(transport)
  const closing = useSessionStore.getState().requestClose(1)
  await Promise.resolve()
  expect(transport.calls).toHaveLength(0)
  old.resolve()
  await closing
  expect(transport.calls.map((call) => call.method)).toEqual(['doc.summary'])
  expect(useUiStore.getState().closePrompt).toEqual({ doc: 1 })
  expect(useSessionStore.getState().docs).toHaveLength(1)
})

it('waits before explicit discard without saving and leaves unrelated editors alone', async () => {
  const old = deferred()
  void holdDocumentEdits(1, old.promise)
  const prepare = vi.fn().mockResolvedValue(undefined)
  editor(2, prepare)
  const transport = createMockTransport({ 'session.close': () => null })
  setTransport(transport)
  const closing = useSessionStore.getState().close(1)
  await Promise.resolve()
  expect(transport.calls).toHaveLength(0)
  old.resolve()
  await closing
  expect(prepare).not.toHaveBeenCalled()
  expect(transport.calls.map((call) => call.method)).toEqual(['session.close'])
})

it('releases the document barrier when persistence fails', async () => {
  await expect(
    withSettledDocument(1, () => Promise.reject(new Error('disk full'))),
  ).rejects.toThrow('disk full')
  await expect(withSettledDocument(1, async () => 'retry')).resolves.toBe(
    'retry',
  )
})

it('commits an unfinished real Pen path before persisting the document', async () => {
  const transport = createMockTransport({
    'doc.apply': () => noChange([42]),
    'session.save': () => summary,
    'session.recent': () => [],
  })
  setTransport(transport)
  const actual = editor(1, async () => undefined)
  actual.setTool('pen')
  for (const at of [
    { x: 100, y: 100 },
    { x: 250, y: 200 },
  ]) {
    const pointer = {
      ...actual.viewportForTest.toScreen(at),
      button: 0,
      modifiers: { alt: false, shift: false, mod: false },
    }
    actual.pointerDown(pointer)
    actual.pointerUp(pointer)
  }
  expect(actual.claimsKey('Enter')).toBe(true)
  expect(await useSessionStore.getState().save(1)).toBe(true)
  const applied = transport.calls.find((call) => call.method === 'doc.apply')
  expect(applied?.params).toMatchObject({
    command: {
      AddContour: {
        glyph: 'a',
        contour: {
          points: [
            { x: 100, y: 100 },
            { x: 250, y: 200 },
          ],
        },
      },
    },
  })
  const methods = transport.calls.map((call) => call.method)
  expect(methods.indexOf('doc.apply')).toBeLessThan(
    methods.indexOf('session.save'),
  )
})
