import { cleanup, renderHook, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'

import { setTransport, type DocId, type EditorSettingsView } from '../../api'
import {
  DEFAULT_PREFS,
  nudgeStepsOf,
  snapSettingsOf,
} from '../../stores/prefsStore'
import { createMockTransport } from '../../test/mockTransport'

import { useFontEditorSettings } from './useFontEditorSettings'

afterEach(() => {
  cleanup()
  setTransport(undefined)
})

/** What the kernel answers for a font that asks for a 20-unit grid. */
function view(): EditorSettingsView {
  return {
    stored: { grid_size: 20, grid_subdivision: 4, nudge: [2, 20, 200] },
    grid_size: 20,
    grid_subdivision: 4,
    nudge: [2, 20, 200],
  }
}

describe('the font decides the grid and the increments', () => {
  const prefs = {
    ...DEFAULT_PREFS,
    grid_size: 1,
    grid_subdivision: 1,
    nudge: [1, 10, 100] as [number, number, number],
  }

  it('uses the preference when the font stores nothing', () => {
    expect(snapSettingsOf(prefs, null).gridSize).toBe(1)
    expect(nudgeStepsOf(prefs, null)).toEqual([1, 10, 100])
  })

  it('uses the font’s own grid, subdivision and increments when present', () => {
    const font = {
      grid_size: 20,
      grid_subdivision: 4,
      nudge: [2, 20, 200] as [number, number, number],
    }
    // The grid the canvas draws and the snap both read one resolved step.
    expect(snapSettingsOf(prefs, font).gridSize).toBe(5)
    expect(nudgeStepsOf(prefs, font)).toEqual([2, 20, 200])
  })

  it('keeps a zero grid meaning “no grid”, whichever side set it', () => {
    expect(snapSettingsOf({ ...prefs, grid_size: 0 }, null).gridSize).toBe(0)
    expect(
      snapSettingsOf(prefs, {
        grid_size: 0,
        grid_subdivision: 1,
        nudge: [1, 10, 100],
      }).gridSize,
    ).toBe(0)
  })

  it('asks the kernel once per document and falls back when it cannot', async () => {
    const transport = createMockTransport({
      'fontinfo.editor_settings_get': () => ({
        stored: { grid_size: 20 },
        grid_size: 20,
        grid_subdivision: 4,
        nudge: [2, 20, 200] as [number, number, number],
      }),
    })
    setTransport(transport)
    const { call } = await import('../../api')
    const view = await call('fontinfo.editor_settings_get', { doc: 1 })
    expect(view.grid_size).toBe(20)
    expect(snapSettingsOf(prefs, view).gridSize).toBe(5)
  })
})

describe('the editor asks each document for its own settings', () => {
  it('reports nothing without a document and the font’s values with one', async () => {
    const transport = createMockTransport({
      'fontinfo.editor_settings_get': () => view(),
    })
    setTransport(transport)
    const { result, rerender } = renderHook(
      ({ doc, generation }: { doc: DocId | null; generation: number }) =>
        useFontEditorSettings(doc, generation),
      { initialProps: { doc: null as DocId | null, generation: 0 } },
    )
    // No document, no question asked and no settings claimed.
    expect(result.current).toBeNull()
    expect(transport.calls).toHaveLength(0)

    rerender({ doc: 1 as DocId, generation: 0 })
    await waitFor(() => expect(result.current?.grid_size).toBe(20))
    expect(result.current?.nudge).toEqual([2, 20, 200])
    expect(snapSettingsOf(DEFAULT_PREFS, result.current).gridSize).toBe(5)

    // A new generation is a new answer: the settings are editable font data.
    // The hook reports nothing until that answer arrives, by design, so the
    // value is awaited rather than read in the same tick as the call.
    rerender({ doc: 1 as DocId, generation: 1 })
    await waitFor(() => expect(transport.calls).toHaveLength(2))
    await waitFor(() => expect(result.current?.grid_size).toBe(20))
  })

  it('leaves the preference in charge when the read fails', async () => {
    const transport = createMockTransport({
      'fontinfo.editor_settings_get': () => {
        throw new Error('no such document')
      },
    })
    setTransport(transport)
    const { result } = renderHook(() => useFontEditorSettings(1 as DocId, 0))
    await waitFor(() => expect(transport.calls).toHaveLength(1))
    expect(result.current).toBeNull()
    // Which is exactly the "font stores nothing" case above.
    expect(snapSettingsOf(DEFAULT_PREFS, result.current).gridSize).toBe(
      snapSettingsOf(DEFAULT_PREFS, null).gridSize,
    )
    expect(nudgeStepsOf(DEFAULT_PREFS, result.current)).toEqual(
      nudgeStepsOf(DEFAULT_PREFS, null),
    )
  })
})
