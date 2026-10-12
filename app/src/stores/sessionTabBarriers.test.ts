import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { setTransport } from '../api'
import * as editors from '../editor/activeEditor'
import type { EditorController } from '../editor/editorController'
import { initialUiState, tabId, useUiStore } from './uiStore'
import { initialSessionState, useSessionStore } from './sessionStore'
import { createMockTransport, makeSummary } from '../test/mockTransport'

function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((yes) => {
    resolve = yes
  })
  return { promise, resolve }
}

beforeEach(() => {
  localStorage.clear()
  useUiStore.setState(initialUiState())
  useSessionStore.setState(initialSessionState())
})

afterEach(() => {
  setTransport(undefined)
  vi.restoreAllMocks()
})

it.each(['open', 'createDocument', 'recover'] as const)(
  '%s drains the previous document before activating the new font',
  async (operation) => {
    const old = useUiStore.getState().openTab({ kind: 'glyph', doc: 1, gid: 9 })
    const closing = deferred<void>()
    const prepareTransition = vi.fn(() => closing.promise)
    vi.spyOn(editors, 'activeEditor').mockReturnValue({
      documentId: 1,
      prepareTransition,
    } as unknown as EditorController)
    const summary = makeSummary({ doc: 2 })
    setTransport(
      createMockTransport({
        'session.open': () => summary,
        'session.new': () => summary,
        'session.recover': () => summary,
        'session.recent': () => [],
      }),
    )
    const opened = useSessionStore.getState()[operation]('/font.fcf')
    expect(prepareTransition).toHaveBeenCalledOnce()
    await Promise.resolve()
    expect(useUiStore.getState().activeTab).toBe(old)
    closing.resolve()
    expect(await opened).toEqual(summary)
    expect(useUiStore.getState().activeTab).toBe(tabId('font', 2))
  },
)

it('keeps a delayed open result in the background after another tab was selected', async () => {
  useUiStore.getState().openTab({ kind: 'font', doc: 1 })
  const opened = deferred<ReturnType<typeof makeSummary>>()
  setTransport(
    createMockTransport({
      'session.open': () => opened.promise,
      'session.recent': () => [],
    }),
  )
  const opening = useSessionStore.getState().open('/font.fcf')
  const other = useUiStore.getState().openTab({ kind: 'font', doc: 3 })
  opened.resolve(makeSummary({ doc: 2 }))
  await opening
  expect(useUiStore.getState().activeTab).toBe(other)
  expect(useUiStore.getState().tabs.some((tab) => tab.doc === 2)).toBe(true)
})

it('settles edits begun while a font open was waiting for the server', async () => {
  const old = useUiStore.getState().openTab({ kind: 'glyph', doc: 1, gid: 9 })
  const opened = deferred<ReturnType<typeof makeSummary>>()
  const closing = deferred<void>()
  const prepareTransition = vi
    .fn(() => closing.promise)
    .mockResolvedValueOnce(undefined)
  vi.spyOn(editors, 'activeEditor').mockReturnValue({
    documentId: 1,
    prepareTransition,
  } as unknown as EditorController)
  setTransport(
    createMockTransport({
      'session.open': () => opened.promise,
      'session.recent': () => [],
    }),
  )
  const opening = useSessionStore.getState().open('/font.fcf')
  await Promise.resolve()
  expect(prepareTransition).toHaveBeenCalledOnce()
  opened.resolve(makeSummary({ doc: 2 }))
  await vi.waitFor(() => expect(prepareTransition).toHaveBeenCalledTimes(2))
  expect(useUiStore.getState().activeTab).toBe(old)
  closing.resolve()
  await opening
  expect(useUiStore.getState().activeTab).toBe(tabId('font', 2))
})
