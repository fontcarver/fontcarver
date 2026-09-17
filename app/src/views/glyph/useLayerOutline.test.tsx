import { act, renderHook, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { setTransport } from '../../api'
import { initialEditorState, useEditorStore } from '../../stores/editorStore'
import {
  makeContour,
  makeLayer,
  makeLayerOutline,
  makePoint,
} from '../../test/fontFixtures'
import {
  createMockTransport,
  type MockTransport,
} from '../../test/mockTransport'

import { useLayerOutline } from './useLayerOutline'

const layer = makeLayer({
  contours: [makeContour([makePoint(0, 0), makePoint(100, 0, 'Line')])],
})

let width = 600

function transport(): MockTransport {
  const mock = createMockTransport({
    'glyphs.outline': () => makeLayerOutline(layer, { width }),
  })
  setTransport(mock)
  return mock
}

function loads(mock: MockTransport): number {
  return mock.calls.filter((entry) => entry.method === 'glyphs.outline').length
}

const change = {
  glyphs: [42],
  font_info: false,
  masters: false,
  layout: false,
  feature_source: false,
  glyph_order: false,
  metrics: false,
  layers: false,
  guides: false,
}

beforeEach(() => {
  width = 600
  useEditorStore.setState(initialEditorState())
})

afterEach(() => setTransport(undefined))

describe('useLayerOutline', () => {
  it('reloads when a document event touches the glyph', async () => {
    const mock = transport()
    const { result } = renderHook(() => useLayerOutline(1, 42, 'layer-1'))
    await waitFor(() => expect(result.current.outline).not.toBeNull())

    width = 700
    act(() =>
      mock.emit({
        doc: 1,
        generation: 2,
        dirty: true,
        undo_label: 'Move 1 point',
        redo_label: null,
        change,
      }),
    )
    await waitFor(() => expect(result.current.outline?.width).toBe(700))
    expect(loads(mock)).toBe(2)
  })

  it('keeps the outline on screen while a reload is in flight', async () => {
    const mock = transport()
    const { result } = renderHook(() => useLayerOutline(1, 42, 'layer-1'))
    await waitFor(() => expect(result.current.outline).not.toBeNull())

    // The canvas — and any gesture living in it — must not be unmounted by a
    // refetch, so the previous answer stays until the new one lands.
    act(() => result.current.reload())
    expect(result.current.outline).not.toBeNull()
    await waitFor(() => expect(loads(mock)).toBe(2))
  })

  it('defers reloads while a gesture is live and takes one when it ends', async () => {
    const mock = transport()
    const { result } = renderHook(() => useLayerOutline(1, 42, 'layer-1'))
    await waitFor(() => expect(result.current.outline).not.toBeNull())

    act(() => useEditorStore.getState().setEditing(true))
    act(() =>
      mock.emit({
        doc: 1,
        generation: 3,
        dirty: true,
        undo_label: 'Move 1 point',
        redo_label: null,
        change,
      }),
    )
    expect(loads(mock)).toBe(1)

    width = 800
    act(() => useEditorStore.getState().setEditing(false))
    await waitFor(() => expect(result.current.outline?.width).toBe(800))
    expect(loads(mock)).toBe(2)
  })
})
