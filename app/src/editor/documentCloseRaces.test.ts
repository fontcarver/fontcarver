import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { setTransport } from '../api'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { createMockTransport, makeSummary } from '../test/mockTransport'
import { registerDocumentEditParticipant } from './documentEditParticipants'
import { noteDocumentEdit } from './editSession'

let unregister = () => {}

function deferred() {
  let resolve!: () => void
  let reject!: (error: Error) => void
  const promise = new Promise<void>((yes, no) => {
    resolve = yes
    reject = no
  })
  return { promise, resolve, reject }
}

beforeEach(() => {
  useSessionStore.setState({
    ...initialSessionState(),
    docs: [makeSummary({ path: '/tmp/source.fcf', dirty: false })],
  })
  useUiStore.setState(initialUiState())
  useUiStore.getState().openTab({ kind: 'fontinfo', doc: 1 })
})

afterEach(() => {
  unregister()
  setTransport(undefined)
})

it.each([false, true])(
  'suspends accepted input through final close and recovers on failure=%s',
  async (fails) => {
    const reading = deferred()
    const closing = deferred()
    let suspended = false
    unregister = registerDocumentEditParticipant(1, {
      flush: async () => undefined,
      setSuspended: (value) => {
        suspended = value
      },
    })
    const transport = createMockTransport({
      'doc.summary': async () => {
        await reading.promise
        return makeSummary({ dirty: false })
      },
      'session.close': async () => {
        await closing.promise
        return null
      },
    })
    setTransport(transport)
    const finished = useSessionStore.getState().requestClose(1)
    await vi.waitFor(() =>
      expect(
        transport.calls.some((call) => call.method === 'doc.summary'),
      ).toBe(true),
    )
    expect(suspended).toBe(true)
    reading.resolve()
    await vi.waitFor(() =>
      expect(
        transport.calls.some((call) => call.method === 'session.close'),
      ).toBe(true),
    )
    expect(suspended).toBe(true)
    if (fails) closing.reject(new Error('close failed'))
    else closing.resolve()
    await finished
    expect(suspended).toBe(false)
    expect(useSessionStore.getState().docs).toHaveLength(fails ? 1 : 0)
    expect(useUiStore.getState().tabs).toHaveLength(fails ? 1 : 0)
  },
)

it('asks again about a draft typed while Save and Close was writing the file', async () => {
  const writing = deferred()
  let draft = false
  let dirty = false
  unregister = registerDocumentEditParticipant(1, {
    flush: async () => {
      if (draft) {
        dirty = true
        draft = false
      }
    },
    hasPendingEdits: () => draft,
  })
  const transport = createMockTransport({
    'session.save': async () => {
      await writing.promise
      return makeSummary({ path: '/tmp/source.fcf', dirty: false })
    },
    'session.recent': () => [],
    'doc.summary': () => makeSummary({ path: '/tmp/source.fcf', dirty }),
    'session.close': () => null,
  })
  setTransport(transport)
  const finished = useSessionStore.getState().saveAndClose(1)
  await vi.waitFor(() =>
    expect(transport.calls.some((call) => call.method === 'session.save')).toBe(
      true,
    ),
  )
  draft = true
  noteDocumentEdit(1)
  writing.resolve()
  await finished
  expect(useUiStore.getState().closePrompt).toEqual({ doc: 1 })
  expect(transport.calls.some((call) => call.method === 'session.close')).toBe(
    false,
  )
  expect(useSessionStore.getState().docs).toHaveLength(1)
})
