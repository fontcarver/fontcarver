import { act, renderHook, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { setTransport, type FontInfoView } from '../../api'
import {
  initialDocumentState,
  useDocumentStore,
} from '../../stores/documentStore'
import { makeFontInfo, makeFontInfoView } from '../../test/fontFixtures'
import { createMockTransport, noChange } from '../../test/mockTransport'
import {
  PROOF_LIBRARY_KEY,
  readProofLibrary,
  type ProofEntry,
  writeProofLibrary,
} from './proofLibrary'
import { useProofLibrary } from './useProofLibrary'

const entry: ProofEntry = {
  id: 'proof-1',
  name: 'Spacing',
  group: 'Latin',
  script: 'Latn',
  language: 'en',
  direction: 'ltr',
  text: 'AV /A.alt',
}
function fontView(entries: ProofEntry[] = []): FontInfoView {
  return makeFontInfoView({
    info: makeFontInfo({
      user_data: writeProofLibrary({ vendor: { untouched: true } }, entries),
    }),
  })
}
function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((done) => {
    resolve = done
  })
  return { promise, resolve }
}

beforeEach(() => {
  useDocumentStore.setState(initialDocumentState())
  useDocumentStore.getState().ensure(1)
})
afterEach(() => setTransport(undefined))

describe('useProofLibrary', () => {
  it('loads saved proofs and writes against freshly read font info', async () => {
    let view = fontView([entry])
    const transport = createMockTransport({
      'doc.font_info': () => view,
      'doc.apply': ({ command }) => {
        if (!('SetFontInfo' in command)) throw new Error('wrong command')
        view = { ...view, info: command.SetFontInfo.info }
        return { ...noChange(), font_info: true }
      },
    })
    setTransport(transport)
    const { result } = renderHook(() => useProofLibrary(1))
    await waitFor(() => expect(result.current.entries).toEqual([entry]))
    view = {
      ...view,
      info: {
        ...view.info,
        family_name: 'Changed elsewhere',
        user_data: { ...(view.info.user_data as object), other: 7 },
      },
    }
    await act(async () =>
      expect(
        await result.current.save({ ...entry, text: '新しい /A.alt' }),
      ).toBe(true),
    )
    await waitFor(() =>
      expect(result.current.entries[0]?.text).toBe('新しい /A.alt'),
    )
    expect(view.info.family_name).toBe('Changed elsewhere')
    expect(view.info.user_data).toMatchObject({
      other: 7,
      vendor: { untouched: true },
    })
    expect(
      transport.calls.filter((call) => call.method === 'doc.apply'),
    ).toHaveLength(1)
  })

  it('serializes concurrent writers and merges each against the latest library', async () => {
    let view = fontView()
    const gate = deferred<void>()
    let applies = 0
    setTransport(
      createMockTransport({
        'doc.font_info': () => view,
        'doc.apply': async ({ command }) => {
          if (!('SetFontInfo' in command)) throw new Error('wrong command')
          applies += 1
          if (applies === 1) await gate.promise
          view = { ...view, info: command.SetFontInfo.info }
          return { ...noChange(), font_info: true }
        },
      }),
    )
    const first = renderHook(() => useProofLibrary(1))
    const second = renderHook(() => useProofLibrary(1))
    await waitFor(() =>
      expect(
        first.result.current.loading || second.result.current.loading,
      ).toBe(false),
    )
    let firstSave!: Promise<boolean>, secondSave!: Promise<boolean>
    act(() => {
      firstSave = first.result.current.save(entry)
      secondSave = second.result.current.save({
        ...entry,
        id: 'proof-2',
        text: '天地',
      })
    })
    await waitFor(() => expect(applies).toBe(1))
    await act(async () => {
      gate.resolve()
      await Promise.all([firstSave, secondSave])
    })
    expect(
      readProofLibrary(view.info.user_data).map((entry) => entry.id),
    ).toEqual(['proof-1', 'proof-2'])
  })

  it('keeps saved data visible when a command fails', async () => {
    const view = fontView([entry])
    setTransport(
      createMockTransport({
        'doc.font_info': () => view,
        'doc.apply': () => {
          throw new Error('write failed')
        },
      }),
    )
    const { result } = renderHook(() => useProofLibrary(1))
    await waitFor(() => expect(result.current.entries).toEqual([entry]))
    await act(async () =>
      expect(await result.current.remove(entry.id)).toBe(false),
    )
    expect(result.current.entries).toEqual([entry])
    expect(result.current.error).toBe('write failed')
    expect(result.current.saving).toBe(false)
  })

  it('removes only the selected proof and refreshes after generation changes and undo events', async () => {
    let view = fontView([entry, { ...entry, id: 'proof-2' }])
    const transport = createMockTransport({
      'doc.font_info': () => view,
      'doc.apply': ({ command }) => {
        if (!('SetFontInfo' in command)) throw new Error('wrong command')
        view = { ...view, info: command.SetFontInfo.info }
        return { ...noChange(), font_info: true }
      },
    })
    setTransport(transport)
    const { result } = renderHook(() => useProofLibrary(1))
    await waitFor(() => expect(result.current.entries).toHaveLength(2))
    await act(async () =>
      expect(await result.current.remove(entry.id)).toBe(true),
    )
    await waitFor(() =>
      expect(result.current.entries.map((entry) => entry.id)).toEqual([
        'proof-2',
      ]),
    )
    view = fontView([entry])
    act(() => useDocumentStore.getState().noteGeneration(1, 2))
    await waitFor(() => expect(result.current.entries).toEqual([entry]))
    view = fontView([])
    act(() =>
      transport.emit({
        kind: 'Document',
        doc: 1,
        generation: 3,
        undo_label: null,
        redo_label: null,
        dirty: true,
        change: { ...noChange(), font_info: true },
      }),
    )
    await waitFor(() => expect(result.current.entries).toEqual([]))
  })

  it('ignores a late response after switching documents', async () => {
    const pending = deferred<FontInfoView>()
    setTransport(
      createMockTransport({
        'doc.font_info': ({ doc }) =>
          doc === 1
            ? pending.promise
            : fontView([{ ...entry, id: 'other-doc' }]),
      }),
    )
    const { result, rerender } = renderHook(({ doc }) => useProofLibrary(doc), {
      initialProps: { doc: 1 },
    })
    rerender({ doc: 2 })
    await waitFor(() => expect(result.current.entries[0]?.id).toBe('other-doc'))
    await act(async () => pending.resolve(fontView([entry])))
    expect(result.current.entries[0]?.id).toBe('other-doc')
  })

  it('keeps an old-document save from replacing the newly selected project', async () => {
    const gate = deferred<void>()
    let firstView = fontView([entry])
    setTransport(
      createMockTransport({
        'doc.font_info': ({ doc }) =>
          doc === 1 ? firstView : fontView([{ ...entry, id: 'other-doc' }]),
        'doc.apply': async ({ command }) => {
          await gate.promise
          if ('SetFontInfo' in command)
            firstView = { ...firstView, info: command.SetFontInfo.info }
          return { ...noChange(), font_info: true }
        },
      }),
    )
    const { result, rerender } = renderHook(({ doc }) => useProofLibrary(doc), {
      initialProps: { doc: 1 },
    })
    await waitFor(() => expect(result.current.loading).toBe(false))
    let saving!: Promise<boolean>
    act(() => {
      saving = result.current.save({ ...entry, text: 'Updated' })
    })
    rerender({ doc: 2 })
    await waitFor(() => expect(result.current.entries[0]?.id).toBe('other-doc'))
    await act(async () => {
      gate.resolve()
      await saving
    })
    expect(result.current.entries[0]?.id).toBe('other-doc')
    expect(result.current.saving).toBe(false)
  })

  it('refuses saving over unknown library versions', async () => {
    const transport = createMockTransport({
      'doc.font_info': () =>
        makeFontInfoView({
          info: makeFontInfo({
            user_data: { [PROOF_LIBRARY_KEY]: { version: 2, entries: [] } },
          }),
        }),
    })
    setTransport(transport)
    const { result } = renderHook(() => useProofLibrary(1))
    await waitFor(() => expect(result.current.error).not.toBeNull())
    await act(async () => expect(await result.current.save(entry)).toBe(false))
    expect(transport.calls.some((call) => call.method === 'doc.apply')).toBe(
      false,
    )
  })
})
