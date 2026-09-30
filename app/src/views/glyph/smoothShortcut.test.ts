import { afterEach, beforeEach, expect, it, vi } from 'vitest'

import { resetCommands, runCommand } from '../../shell/commands'
import { commandForEvent } from '../../shell/shortcuts'
import { selectionOf, type Selection } from '../../editor/selection'
import { initialUiState, useUiStore } from '../../stores/uiStore'

import { registerGlyphViewCommands } from './glyphCommands'

const editor = vi.hoisted(() => {
  const emptySelection = (): Selection => new Map()
  return {
    currentSelection: emptySelection(),
    claimsKey: vi.fn(() => false),
    toggleSmooth: vi.fn(),
  }
})
vi.mock('../../editor/activeEditor', () => ({ activeEditor: () => editor }))

let canvas: HTMLCanvasElement
beforeEach(() => {
  resetCommands()
  useUiStore.setState(initialUiState())
  useUiStore.getState().openTab({ kind: 'glyph', doc: 1, gid: 42 })
  editor.claimsKey.mockReturnValue(false)
  editor.toggleSmooth.mockClear()
  editor.currentSelection = selectionOf([
    { kind: 'node', contour: 0, point: 0 },
  ])
  canvas = document.createElement('canvas')
  canvas.tabIndex = 0
  document.body.append(canvas)
  canvas.focus()
  registerGlyphViewCommands()
})
afterEach(() => {
  canvas.remove()
  resetCommands()
})

it('dispatches Alt+Return to the outline-preserving smooth toggle', () => {
  const event = new KeyboardEvent('keydown', { key: 'Enter', altKey: true })
  const command = commandForEvent(event)
  expect(command).toBe('path.toggleSmoothKeepOutline')
  if (command) runCommand(command)
  expect(editor.toggleSmooth).toHaveBeenCalledWith(true)
})

it('leaves Alt+Return to a tool that owns Return', () => {
  editor.claimsKey.mockReturnValue(true)
  const event = new KeyboardEvent('keydown', { key: 'Enter', altKey: true })
  expect(commandForEvent(event)).toBeUndefined()
  expect(editor.toggleSmooth).not.toHaveBeenCalled()
})
