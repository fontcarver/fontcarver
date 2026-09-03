import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { registerBuiltinCommands } from '../../shell/builtinCommands'
import {
  isCommandEnabled,
  resetCommands,
  runCommand,
} from '../../shell/commands'
import {
  CELL_SIZE_DEFAULT,
  initialDocumentState,
  useDocumentStore,
} from '../../stores/documentStore'
import { initialEditorState, useEditorStore } from '../../stores/editorStore'
import { initialUiState, useUiStore } from '../../stores/uiStore'

import {
  activeGlyphView,
  registerGlyphView,
  zoomActiveGlyphView,
} from './glyphCommands'

function mountedView(doc = 1, gid = 42) {
  const handle = {
    doc,
    gid,
    fit: vi.fn(),
    zoomBy: vi.fn(),
    step: vi.fn(),
  }
  const dispose = registerGlyphView(handle)
  return { handle, dispose }
}

beforeEach(() => {
  localStorage.clear()
  resetCommands()
  useUiStore.setState(initialUiState())
  useDocumentStore.setState(initialDocumentState())
  useEditorStore.setState(initialEditorState())
  registerBuiltinCommands()
})

afterEach(() => resetCommands())

describe('glyph view commands', () => {
  it('only reports the mounted view while its own tab is active', () => {
    const { handle, dispose } = mountedView()
    expect(activeGlyphView()).toBeNull()

    useUiStore.getState().openTab({ kind: 'glyph', doc: 1, gid: 42 })
    expect(activeGlyphView()).toBe(handle)

    useUiStore.getState().openTab({ kind: 'font', doc: 1 })
    expect(activeGlyphView()).toBeNull()

    useUiStore.getState().openTab({ kind: 'glyph', doc: 1, gid: 42 })
    dispose()
    expect(activeGlyphView()).toBeNull()
  })

  it('sends the zoom commands to the canvas on a glyph tab', () => {
    const { handle } = mountedView()
    useUiStore.getState().openTab({ kind: 'glyph', doc: 1, gid: 42 })

    expect(runCommand('view.zoomIn')).toBe(true)
    expect(handle.zoomBy).toHaveBeenCalledWith(1.25)
    expect(runCommand('view.zoomOut')).toBe(true)
    expect(handle.zoomBy).toHaveBeenCalledWith(1 / 1.25)
    // The grid's cell size is untouched.
    expect(useDocumentStore.getState().byDoc[1]?.cellSize).toBeUndefined()
  })

  it('leaves the zoom commands stepping the cell size on a font tab', () => {
    mountedView()
    useUiStore.getState().openTab({ kind: 'font', doc: 1 })

    expect(zoomActiveGlyphView(1)).toBe(false)
    runCommand('view.zoomIn')
    expect(useDocumentStore.getState().byDoc[1]?.cellSize).toBeGreaterThan(
      CELL_SIZE_DEFAULT,
    )
  })

  it('fits and steps through glyphs', () => {
    const { handle } = mountedView()
    useUiStore.getState().openTab({ kind: 'glyph', doc: 1, gid: 42 })

    runCommand('view.zoomFit')
    expect(handle.fit).toHaveBeenCalled()
    runCommand('glyph.next')
    expect(handle.step).toHaveBeenCalledWith(1)
    runCommand('glyph.previous')
    expect(handle.step).toHaveBeenCalledWith(-1)
  })

  it('disables the glyph commands away from a glyph tab', () => {
    mountedView()
    useUiStore.getState().openTab({ kind: 'font', doc: 1 })
    for (const id of [
      'view.zoomFit',
      'glyph.next',
      'glyph.previous',
      'view.glyph.points',
    ])
      expect(isCommandEnabled(id)).toBe(false)
  })

  it('toggles the overlays through the command registry', () => {
    mountedView()
    useUiStore.getState().openTab({ kind: 'glyph', doc: 1, gid: 42 })

    runCommand('view.glyph.filled')
    expect(useEditorStore.getState().overlays.filled).toBe(true)
    runCommand('view.glyph.rulers')
    expect(useEditorStore.getState().overlays.rulers).toBe(false)
  })
})
