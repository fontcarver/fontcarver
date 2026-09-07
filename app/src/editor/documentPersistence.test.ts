import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { setTransport } from '../api'
import {
  createMockTransport,
  makeSummary,
  noChange,
} from '../test/mockTransport'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { TextMetricsRunner } from '../views/text/textMetrics'
import { registerEditor } from './activeEditor'
import type { EditorController } from './editorController'
import {
  EditSession,
  holdDocumentEdits,
  immediateScheduler,
} from './editSession'
import { withSettledDocument } from './documentPersistence'

let unregister = () => undefined as void
const summary = makeSummary({ path: '/tmp/persistence.fcf', dirty: false })

function deferred() {
  let resolve!: () => void
  const promise = new Promise<void>((done) => {
    resolve = done
  })
  return { promise, resolve }
}

function editor(doc: number, prepareTransition: () => Promise<void>) {
  unregister = registerEditor({
    frame: () => ({ source: { doc } }),
    prepareTransition,
  } as unknown as EditorController)
}

beforeEach(() => {
  useSessionStore.setState({ ...initialSessionState(), docs: [summary] })
  useUiStore.setState(initialUiState())
})
afterEach(() => {
  unregister()
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
