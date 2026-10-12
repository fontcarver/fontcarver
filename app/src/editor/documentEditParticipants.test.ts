import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { setTransport } from '../api'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { createMockTransport, makeSummary } from '../test/mockTransport'
import { registerBuiltinCommands } from '../shell/builtinCommands'
import {
  isCommandEnabled,
  resetCommands,
  runCommand,
  setCommandErrorReporter,
} from '../shell/commands'
import { closeTab, selectTab } from '../shell/tabActions'
import { commandForEvent } from '../shell/shortcuts'
import * as editors from './activeEditor'
import type { EditorController } from './editorController'
import { afterDocumentEdits, noteDocumentEdit } from './editSession'
import { registerDocumentEditParticipant } from './documentEditParticipants'
import { settleDocumentEdits, withSettledDocument } from './documentPersistence'

const disposers: (() => void)[] = []

function deferred() {
  let resolve!: () => void
  let reject!: (error: Error) => void
  const promise = new Promise<void>((yes, no) => {
    resolve = yes
    reject = no
  })
  return { promise, resolve, reject }
}

function participant(doc: number, flush: () => Promise<void>, dirty = false) {
  disposers.push(
    registerDocumentEditParticipant(doc, {
      flush,
      hasPendingEdits: () => dirty,
    }),
  )
}

function tabs() {
  const old = useUiStore.getState().openTab({ kind: 'fontinfo', doc: 1 })
  const next = useUiStore.getState().openTab({ kind: 'font', doc: 2 })
  useUiStore.getState().setActiveTab(old)
  return { old, next }
}

beforeEach(() => {
  useUiStore.setState(initialUiState())
  useSessionStore.setState({
    ...initialSessionState(),
    docs: [makeSummary({ path: '/tmp/draft.fcf' })],
  })
  resetCommands()
  registerBuiltinCommands()
})

afterEach(() => {
  for (const dispose of disposers.splice(0)) dispose()
  setTransport(undefined)
  setCommandErrorReporter(() => undefined)
  resetCommands()
  vi.restoreAllMocks()
})

it('keeps a tab mounted until a registered draft is persisted', async () => {
  const { old, next } = tabs()
  const saving = deferred()
  const flush = vi.fn(() => saving.promise)
  participant(1, flush)
  const switched = selectTab(next)
  await vi.waitFor(() => expect(flush).toHaveBeenCalledOnce())
  expect(useUiStore.getState().activeTab).toBe(old)
  saving.resolve()
  expect(await switched).toBe(true)
  expect(useUiStore.getState().activeTab).toBe(next)
})

it('retains the active tab and reports a rejected draft flush', async () => {
  const { old, next } = tabs()
  const failure = new Error('draft persistence failed')
  const report = vi.fn()
  setCommandErrorReporter(report)
  participant(1, async () => {
    throw failure
  })
  expect(await selectTab(next)).toBe(false)
  await closeTab(old)
  expect(useUiStore.getState().activeTab).toBe(old)
  expect(useUiStore.getState().tabs.some((tab) => tab.id === old)).toBe(true)
  expect(report).toHaveBeenCalledWith(failure)
})

it('persists another local revision staged while a flush is waiting', async () => {
  const { old, next } = tabs()
  const first = deferred()
  const second = deferred()
  const flush = vi
    .fn(() => second.promise)
    .mockImplementationOnce(() => first.promise)
  participant(1, flush)
  const switched = selectTab(next)
  await vi.waitFor(() => expect(flush).toHaveBeenCalledOnce())
  noteDocumentEdit(1)
  first.resolve()
  await vi.waitFor(() => expect(flush).toHaveBeenCalledTimes(2))
  expect(useUiStore.getState().activeTab).toBe(old)
  second.resolve()
  expect(await switched).toBe(true)
})

it('finishes the controller before another participant opens a transaction', async () => {
  const closing = deferred()
  vi.spyOn(editors, 'activeEditor').mockReturnValue({
    documentId: 1,
    prepareTransition: () => closing.promise,
  } as unknown as EditorController)
  const flush = vi.fn(async () => undefined)
  const save = vi.fn(async () => 'saved')
  participant(1, flush)
  const saving = withSettledDocument(1, save)
  await Promise.resolve()
  expect(flush).not.toHaveBeenCalled()
  expect(save).not.toHaveBeenCalled()
  closing.resolve()
  expect(await saving).toBe('saved')
  expect(flush).toHaveBeenCalledOnce()
})

it('repeats a slow draft flush before the persistence operation', async () => {
  const first = deferred()
  const second = deferred()
  const flush = vi
    .fn(() => second.promise)
    .mockImplementationOnce(() => first.promise)
  participant(1, flush)
  const save = vi.fn(async () => 'saved')
  const saving = withSettledDocument(1, save)
  await vi.waitFor(() => expect(flush).toHaveBeenCalledOnce())
  noteDocumentEdit(1)
  first.resolve()
  await vi.waitFor(() => expect(flush).toHaveBeenCalledTimes(2))
  expect(save).not.toHaveBeenCalled()
  second.resolve()
  expect(await saving).toBe('saved')
})

it('propagates a failed flush and permits an explicit retry', async () => {
  const flush = vi
    .fn(async () => undefined)
    .mockRejectedValueOnce(new Error('offline'))
  const save = vi.fn(async () => 'saved')
  participant(1, flush)
  await expect(withSettledDocument(1, save)).rejects.toThrow('offline')
  expect(save).not.toHaveBeenCalled()
  expect(await withSettledDocument(1, save)).toBe('saved')
})

it('does not run save, export, history or close after draft persistence fails', async () => {
  const { old } = tabs()
  const report = vi.fn()
  setCommandErrorReporter(report)
  participant(
    1,
    async () => {
      throw new Error('offline')
    },
    true,
  )
  const transport = createMockTransport({})
  setTransport(transport)
  expect(await useSessionStore.getState().save(1)).toBe(false)
  const exportFont = vi.fn(async () => undefined)
  await expect(withSettledDocument(1, exportFont)).rejects.toThrow('offline')
  expect(isCommandEnabled('edit.undo')).toBe(true)
  expect(runCommand('edit.undo')).toBe(true)
  await vi.waitFor(() => expect(report).toHaveBeenCalledOnce())
  await useSessionStore.getState().requestClose(1)
  expect(exportFont).not.toHaveBeenCalled()
  expect(transport.calls).toEqual([])
  expect(useUiStore.getState().activeTab).toBe(old)
  expect(useSessionStore.getState().docs).toHaveLength(1)
})

it('lets the document own undo after persisting the local draft', async () => {
  tabs()
  const persisted = deferred()
  const flush = vi.fn(() => persisted.promise)
  participant(1, flush, true)
  const transport = createMockTransport({
    'doc.undo': () => null,
    'doc.summary': () => makeSummary(),
  })
  setTransport(transport)
  expect(isCommandEnabled('edit.undo')).toBe(true)
  expect(runCommand('edit.undo')).toBe(true)
  await vi.waitFor(() => expect(flush).toHaveBeenCalledOnce())
  expect(transport.calls).toEqual([])
  persisted.resolve()
  await vi.waitFor(() =>
    expect(transport.calls.some((call) => call.method === 'doc.undo')).toBe(
      true,
    ),
  )
})

it('drains participants added during settlement without touching other documents', async () => {
  const pending = deferred()
  const first = vi.fn(() => pending.promise)
  const second = vi.fn(async () => undefined)
  const unrelated = vi.fn(async () => undefined)
  participant(1, first)
  participant(2, unrelated)
  const settling = settleDocumentEdits(1)
  await vi.waitFor(() => expect(first).toHaveBeenCalledOnce())
  participant(1, second)
  pending.resolve()
  await settling
  expect(second).toHaveBeenCalled()
  expect(unrelated).not.toHaveBeenCalled()
})

it('serializes multiple participants across concurrent handovers', async () => {
  const first = deferred()
  const second = deferred()
  const flushFirst = vi.fn(() => first.promise)
  const flushSecond = vi.fn(() => second.promise)
  participant(1, flushFirst)
  participant(1, flushSecond)
  const one = settleDocumentEdits(1)
  const two = settleDocumentEdits(1)
  await vi.waitFor(() => expect(flushFirst).toHaveBeenCalledOnce())
  expect(flushSecond).not.toHaveBeenCalled()
  first.resolve()
  await vi.waitFor(() => expect(flushSecond).toHaveBeenCalledOnce())
  second.resolve()
  await Promise.all([one, two])
  expect(flushFirst).toHaveBeenCalledOnce()
  expect(flushSecond).toHaveBeenCalledOnce()
})

it('routes Save from contenteditable without claiming its native typing history', () => {
  tabs()
  const editable = document.createElement('div')
  Object.defineProperty(editable, 'isContentEditable', { value: true })
  const save = new KeyboardEvent('keydown', { key: 's', ctrlKey: true })
  Object.defineProperty(save, 'target', { value: editable })
  expect(commandForEvent(save, false)).toBe('file.save')
  const undo = new KeyboardEvent('keydown', { key: 'z', ctrlKey: true })
  Object.defineProperty(undo, 'target', { value: editable })
  expect(commandForEvent(undo, false)).toBeUndefined()
})

it('holds later document commands behind a registered participant transaction', async () => {
  const persisting = deferred()
  const flush = vi.fn(() => persisting.promise)
  participant(1, flush)
  const settling = settleDocumentEdits(1)
  await vi.waitFor(() => expect(flush).toHaveBeenCalledOnce())
  const next = vi.fn(async () => undefined)
  const editing = afterDocumentEdits(1, next)
  await Promise.resolve()
  expect(next).not.toHaveBeenCalled()
  persisting.resolve()
  await Promise.all([settling, editing])
  expect(next).toHaveBeenCalledOnce()
})

it('re-settles a controller revision before a participant can mutate the document', async () => {
  const first = deferred()
  const second = deferred()
  const prepare = vi
    .fn(() => second.promise)
    .mockImplementationOnce(() => first.promise)
  vi.spyOn(editors, 'activeEditor').mockReturnValue({
    documentId: 1,
    prepareTransition: prepare,
  } as unknown as EditorController)
  const flush = vi.fn(async () => undefined)
  participant(1, flush)
  const settling = settleDocumentEdits(1)
  noteDocumentEdit(1)
  first.resolve()
  await vi.waitFor(() => expect(prepare).toHaveBeenCalledTimes(2))
  expect(flush).not.toHaveBeenCalled()
  second.resolve()
  await settling
})

it('includes a new controller draft staged during save preparation', async () => {
  const first = deferred()
  const second = deferred()
  const prepare = vi
    .fn(() => second.promise)
    .mockImplementationOnce(() => first.promise)
  vi.spyOn(editors, 'activeEditor').mockReturnValue({
    documentId: 1,
    prepareTransition: prepare,
  } as unknown as EditorController)
  const save = vi.fn(async () => undefined)
  const saving = withSettledDocument(1, save)
  noteDocumentEdit(1)
  first.resolve()
  await vi.waitFor(() => expect(prepare).toHaveBeenCalledTimes(2))
  expect(save).not.toHaveBeenCalled()
  second.resolve()
  await saving
})

it('rechecks a clean close summary after a new local draft arrives', async () => {
  const { old } = tabs()
  const reading = deferred()
  let staged = false
  let dirty = false
  const flush = vi.fn(async () => {
    if (staged) {
      dirty = true
      staged = false
    }
  })
  participant(1, flush)
  let reads = 0
  const transport = createMockTransport({
    'doc.summary': async () => {
      const captured = dirty
      if (++reads === 1) await reading.promise
      return makeSummary({ dirty: captured })
    },
    'session.close': () => null,
  })
  setTransport(transport)
  const closing = useSessionStore.getState().requestClose(1)
  await vi.waitFor(() => expect(reads).toBe(1))
  staged = true
  noteDocumentEdit(1)
  reading.resolve()
  await closing
  expect(flush.mock.calls.length).toBeGreaterThan(1)
  expect(useUiStore.getState().closePrompt).toEqual({ doc: 1 })
  expect(useUiStore.getState().activeTab).toBe(old)
  expect(transport.calls.some((call) => call.method === 'session.close')).toBe(
    false,
  )
})

it('serializes persistence operations and prepares each one after the preceding save', async () => {
  const saving = deferred()
  const first = vi.fn(() => saving.promise)
  const second = vi.fn(async () => undefined)
  const flush = vi.fn(async () => undefined)
  participant(1, flush)
  const one = withSettledDocument(1, first)
  const two = withSettledDocument(1, second)
  await vi.waitFor(() => expect(first).toHaveBeenCalledOnce())
  expect(second).not.toHaveBeenCalled()
  expect(flush).toHaveBeenCalledOnce()
  saving.resolve()
  await Promise.all([one, two])
  expect(second).toHaveBeenCalledOnce()
  expect(flush).toHaveBeenCalledTimes(2)
})
