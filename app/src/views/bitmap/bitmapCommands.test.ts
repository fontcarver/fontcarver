import { beforeEach, describe, expect, it } from 'vitest'
import { commandForEvent } from '../../shell/shortcuts'
import { getCommand, commandsForMenu, runCommand } from '../../shell/commands'
import { registerBuiltinCommands } from '../../shell/builtinCommands'
import {
  PIXEL_OVERLAY_KEYS,
  PIXEL_TOOL_KEYS,
  registerBitmapCommands,
} from './bitmapCommands'
import { readOverlays, useBitmapToolStore } from './bitmapToolStore'

function keyEvent(key: string): KeyboardEvent {
  return new KeyboardEvent('keydown', { key })
}

beforeEach(() => {
  localStorage.clear()
  registerBuiltinCommands()
  registerBitmapCommands()
  useBitmapToolStore.setState({
    tool: 'pencil',
    overlays: readOverlays(),
    editors: 0,
  })
})

describe('bitmap tool and overlay commands', () => {
  it('registers every tool and overlay in the Bitmap menu', () => {
    const ids = commandsForMenu('bitmap').map((command) => command.id)
    for (const { tool } of PIXEL_TOOL_KEYS)
      expect(ids).toContain(`bitmap.tool.${tool}`)
    for (const { overlay } of PIXEL_OVERLAY_KEYS)
      expect(ids).toContain(`bitmap.overlay.${overlay}`)
  })

  it('gives every tool a distinct shortcut that no other command claims', () => {
    const shortcuts = PIXEL_TOOL_KEYS.map(({ shortcut }) => shortcut)
    expect(new Set(shortcuts).size).toBe(shortcuts.length)
    // Nothing outside the bitmap tools binds a bare letter key.
    for (const { tool, shortcut } of PIXEL_TOOL_KEYS) {
      const command = getCommand(`bitmap.tool.${tool}`)
      expect(command?.shortcut).toBe(shortcut)
    }
  })

  it('binds the tool keys only while a strike editor is mounted', () => {
    expect(commandForEvent(keyEvent('b'))).toBeUndefined()
    const release = useBitmapToolStore.getState().registerEditor()
    expect(commandForEvent(keyEvent('b'))).toBe('bitmap.tool.pencil')
    expect(commandForEvent(keyEvent('m'))).toBe('bitmap.tool.select')
    expect(commandForEvent(keyEvent('i'))).toBe('bitmap.tool.eyedropper')
    release()
    expect(commandForEvent(keyEvent('b'))).toBeUndefined()
  })

  it('selects the tool and toggles the overlay the command names', () => {
    const release = useBitmapToolStore.getState().registerEditor()
    expect(runCommand('bitmap.tool.ellipse')).toBe(true)
    expect(useBitmapToolStore.getState().tool).toBe('ellipse')
    expect(runCommand('bitmap.overlay.grid')).toBe(true)
    expect(useBitmapToolStore.getState().overlays.grid).toBe(false)
    expect(readOverlays().grid).toBe(false)
    release()
    // With no editor open the command refuses rather than changing state.
    expect(runCommand('bitmap.tool.pencil')).toBe(false)
    expect(useBitmapToolStore.getState().tool).toBe('ellipse')
  })
})
