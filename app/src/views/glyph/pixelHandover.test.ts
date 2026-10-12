import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { initialEditorState, useEditorStore } from '../../stores/editorStore'

import { handOverToStrikeEditor, toolAfterPixel } from './pixelHandover'

const openBitmapTab = vi.hoisted(() => vi.fn(() => Promise.resolve('bitmap:1')))

vi.mock('../../shell/tabActions', () => ({ openBitmapTab }))

describe('the Pixel tool hands the glyph over to the strike editor', () => {
  beforeEach(() => {
    openBitmapTab.mockClear()
    useEditorStore.setState(initialEditorState())
  })

  afterEach(() => {
    useEditorStore.setState(initialEditorState())
  })

  it('opens the bitmap workspace for the edited document', () => {
    useEditorStore.getState().setTool('pixel')
    handOverToStrikeEditor(7, 'pen')
    expect(openBitmapTab).toHaveBeenCalledWith(7)
  })

  it('puts the tool the user had back, so the Edit tab keeps a live tool', () => {
    useEditorStore.getState().setTool('pixel')
    handOverToStrikeEditor(7, 'pencil')
    expect(useEditorStore.getState().tool).toBe('pencil')
    expect(useEditorStore.getState().borrowedTool).toBeNull()
  })

  it('releases a borrowed Pixel key without changing the chosen tool', () => {
    useEditorStore.getState().setTool('knife')
    useEditorStore.getState().borrowTool('pixel')
    handOverToStrikeEditor(2, 'knife')
    expect(useEditorStore.getState().tool).toBe('knife')
    expect(useEditorStore.getState().borrowedTool).toBeNull()
    expect(openBitmapTab).toHaveBeenCalledWith(2)
  })

  it('falls back to Select when Pixel was already the chosen tool', () => {
    expect(toolAfterPixel('pixel')).toBe('select')
    expect(toolAfterPixel('measure')).toBe('measure')
  })
})
