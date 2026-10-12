import { act, cleanup, renderHook, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { setTransport, type ApiMethods, type Image } from '../../api'
import type { LayerTarget } from '../../editor/pointCommands'
import { initialEditorState, useEditorStore } from '../../stores/editorStore'
import { createMockTransport, noChange } from '../../test/mockTransport'

import { useSourceImage } from './useSourceImage'

type ImageState = ApiMethods['image.state']['result']

const target: LayerTarget = { doc: 1, glyph: 'A', layer: 'regular' }
const source: Image = {
  file_name: 'drawing.png',
  data: null,
  transform: [1, 0, 0, 1, 10, 20],
  color: null,
  locked: false,
}

function state(overrides: Partial<ImageState> = {}): ImageState {
  return {
    image: source,
    png: [137, 80, 78, 71],
    width: 100,
    height: 60,
    generation: 1,
    error: null,
    ...overrides,
  }
}

function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((done) => {
    resolve = done
  })
  return { promise, resolve }
}

let bitmaps: HTMLImageElement[]
let revoke: ReturnType<typeof vi.fn>
let create: ReturnType<typeof vi.fn>

beforeEach(() => {
  useEditorStore.setState(initialEditorState())
  bitmaps = []
  vi.stubGlobal(
    'Image',
    class {
      constructor() {
        const bitmap = document.createElement('img')
        bitmaps.push(bitmap)
        return bitmap
      }
    },
  )
  create = vi.fn(() => `blob:source-${bitmaps.length}`)
  revoke = vi.fn()
  vi.stubGlobal(
    'URL',
    Object.assign(class extends URL {}, {
      createObjectURL: create,
      revokeObjectURL: revoke,
    }),
  )
})

afterEach(() => {
  cleanup()
  setTransport(undefined)
  vi.unstubAllGlobals()
})

async function decoded(index = 0) {
  await waitFor(() => expect(bitmaps[index]).toBeDefined())
  act(() => bitmaps[index].dispatchEvent(new Event('load')))
}

describe('useSourceImage', () => {
  it('loads the resolved PNG and uses the kernel image placement', async () => {
    const placement: Image = { ...source, transform: [2, 0, 0, 3, 40, 50] }
    const mock = createMockTransport({
      'image.state': () => state({ image: placement }),
    })
    setTransport(mock)
    const { result, unmount } = renderHook(() => useSourceImage(target, source))

    expect(result.current).toBeNull()
    await decoded()

    expect(mock.calls).toEqual([{ method: 'image.state', params: target }])
    expect(create.mock.calls[0][0]).toBeInstanceOf(Blob)
    expect((create.mock.calls[0][0] as Blob).type).toBe('image/png')
    expect(result.current).toEqual({
      bitmap: bitmaps[0],
      width: 100,
      height: 60,
      transform: placement.transform,
    })
    expect(revoke).toHaveBeenCalledTimes(1)
    unmount()
    expect(revoke).toHaveBeenCalledTimes(1)
  })

  it('does not request images for a missing target or source', () => {
    const mock = createMockTransport()
    setTransport(mock)
    const { rerender } = renderHook(
      ({ selected, image }) => useSourceImage(selected, image),
      {
        initialProps: {
          selected: null as LayerTarget | null,
          image: source as Image | null,
        },
      },
    )
    rerender({ selected: target, image: null })
    expect(mock.calls).toHaveLength(0)
  })

  it('ignores a response from a target that is no longer selected', async () => {
    const old = deferred<ImageState>()
    const mock = createMockTransport({
      'image.state': ({ glyph }) => (glyph === 'A' ? old.promise : state()),
    })
    setTransport(mock)
    const { result, rerender } = renderHook(
      ({ selected }) => useSourceImage(selected, source),
      { initialProps: { selected: target } },
    )
    rerender({ selected: { ...target, glyph: 'B' } })
    await decoded()
    const current = result.current
    await act(async () => old.resolve(state({ width: 900 })))
    expect(result.current).toBe(current)
    expect(bitmaps).toHaveLength(1)
  })

  it('releases a pending decode and never paints it after changing targets', async () => {
    const mock = createMockTransport({ 'image.state': () => state() })
    setTransport(mock)
    const { result, rerender, unmount } = renderHook(
      ({ selected }) => useSourceImage(selected, source),
      { initialProps: { selected: target } },
    )
    await waitFor(() => expect(bitmaps).toHaveLength(1))
    const lateLoad = bitmaps[0].onload
    rerender({ selected: { ...target, glyph: 'B' } })
    expect(revoke).toHaveBeenCalledWith('blob:source-0')
    expect(bitmaps[0].onload).toBeNull()
    expect(bitmaps[0].onerror).toBeNull()
    act(() => lateLoad?.call(bitmaps[0], new Event('load')))
    expect(result.current).toBeNull()
    await waitFor(() => expect(bitmaps).toHaveLength(2))
    unmount()
    expect(revoke).toHaveBeenCalledTimes(2)
  })

  it('hides a decoded image immediately when its source is removed', async () => {
    setTransport(createMockTransport({ 'image.state': () => state() }))
    const { result, rerender } = renderHook(
      ({ image }) => useSourceImage(target, image),
      { initialProps: { image: source as Image | null } },
    )
    await decoded()
    expect(result.current).not.toBeNull()
    rerender({ image: null })
    expect(result.current).toBeNull()
  })

  it('refreshes broken links for its document and avoids equivalent metadata reloads', async () => {
    let available = false
    const mock = createMockTransport({
      'image.state': () =>
        available
          ? state()
          : state({
              png: null,
              width: null,
              height: null,
              error: 'Missing file',
            }),
    })
    setTransport(mock)
    const { result, rerender } = renderHook(
      ({ image, refreshKey }) =>
        useSourceImage({ ...target }, image, refreshKey),
      { initialProps: { image: source, refreshKey: 0 } },
    )
    await waitFor(() => expect(mock.calls).toHaveLength(1))
    rerender({ image: { ...source }, refreshKey: 0 })
    act(() =>
      mock.emit({
        doc: 2,
        generation: 2,
        dirty: true,
        undo_label: null,
        redo_label: null,
        change: noChange(),
      }),
    )
    expect(mock.calls).toHaveLength(1)
    available = true
    act(() =>
      mock.emit({
        doc: 1,
        generation: 2,
        dirty: true,
        undo_label: null,
        redo_label: null,
        change: noChange(),
      }),
    )
    await decoded()
    expect(result.current).not.toBeNull()
    expect(mock.calls).toHaveLength(2)
    rerender({ image: { ...source }, refreshKey: 1 })
    await waitFor(() => expect(mock.calls).toHaveLength(3))
  })

  it('reloads replacement image metadata for the same layer', async () => {
    const mock = createMockTransport({ 'image.state': () => state() })
    setTransport(mock)
    const { result, rerender } = renderHook(
      ({ image }) => useSourceImage(target, image),
      { initialProps: { image: source } },
    )
    await decoded()
    rerender({ image: { ...source, file_name: 'replacement.png' } })
    expect(result.current).toBeNull()
    await waitFor(() => expect(mock.calls).toHaveLength(2))
  })

  it('coalesces document refreshes during a gesture until editing finishes', async () => {
    const mock = createMockTransport({ 'image.state': () => state() })
    setTransport(mock)
    const { result } = renderHook(() => useSourceImage(target, source))
    await decoded()
    const original = result.current
    act(() => useEditorStore.getState().setEditing(true))
    for (const generation of [2, 3]) {
      act(() =>
        mock.emit({
          doc: 1,
          generation,
          dirty: true,
          undo_label: 'Move points',
          redo_label: null,
          change: noChange([42]),
        }),
      )
    }
    expect(mock.calls).toHaveLength(1)
    expect(result.current).toBe(original)
    act(() => useEditorStore.getState().setEditing(false))
    await decoded(1)
    expect(mock.calls).toHaveLength(2)
    expect(result.current?.bitmap).toBe(bitmaps[1])
  })

  it('releases failed PNG decodes and leaves no drawable image', async () => {
    setTransport(createMockTransport({ 'image.state': () => state() }))
    const { result } = renderHook(() => useSourceImage(target, source))
    await waitFor(() => expect(bitmaps).toHaveLength(1))
    act(() => bitmaps[0].dispatchEvent(new Event('error')))
    expect(result.current).toBeNull()
    expect(revoke).toHaveBeenCalledTimes(1)
    expect(bitmaps[0].onload).toBeNull()
    expect(bitmaps[0].onerror).toBeNull()
  })

  it('silently clears a prior image after a failed refresh', async () => {
    let failed = false
    const mock = createMockTransport({
      'image.state': () => {
        if (failed) throw new Error('Document closed')
        return state()
      },
    })
    setTransport(mock)
    const { result, rerender } = renderHook(
      ({ refreshKey }) => useSourceImage(target, source, refreshKey),
      { initialProps: { refreshKey: 0 } },
    )
    await decoded()
    failed = true
    rerender({ refreshKey: 1 })
    await waitFor(() => expect(result.current).toBeNull())
  })
})
