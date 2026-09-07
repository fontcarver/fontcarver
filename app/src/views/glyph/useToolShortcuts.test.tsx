import { act, render } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import { HOLD_BORROW_MS, TOOLS } from '../../editor/tools/registry'
import { initialEditorState, useEditorStore } from '../../stores/editorStore'
import { initialUiState, useUiStore } from '../../stores/uiStore'

import { useToolShortcuts } from './useToolShortcuts'

function Host() {
  useToolShortcuts(true)
  return <div />
}

/** Presses a tool key and releases it after `hold` milliseconds. */
function press(key: string, options: { shift?: boolean; hold?: number } = {}) {
  const init = { key, shiftKey: options.shift ?? false, bubbles: true }
  act(() => {
    window.dispatchEvent(new KeyboardEvent('keydown', init))
  })
  if (options.hold !== undefined)
    act(() => {
      vi.advanceTimersByTime(options.hold as number)
    })
  act(() => {
    window.dispatchEvent(new KeyboardEvent('keyup', init))
  })
}

beforeEach(() => {
  useEditorStore.setState(initialEditorState())
  useUiStore.setState(initialUiState())
})

describe('useToolShortcuts', () => {
  it('switches on a tap for every key of the §5 table', () => {
    vi.useFakeTimers()
    render(<Host />)
    try {
      for (const tool of TOOLS) {
        press(tool.key, { shift: tool.shift })
        expect(useEditorStore.getState().tool, tool.id).toBe(tool.id)
        expect(useEditorStore.getState().borrowedTool).toBeNull()
      }
    } finally {
      vi.useRealTimers()
    }
  })

  it('borrows a tool while its key is held and gives it back on release', () => {
    vi.useFakeTimers()
    render(<Host />)
    try {
      useEditorStore.getState().setTool('select')
      act(() => {
        window.dispatchEvent(new KeyboardEvent('keydown', { key: 'p' }))
      })
      expect(useEditorStore.getState().borrowedTool).toBeNull()
      act(() => {
        vi.advanceTimersByTime(HOLD_BORROW_MS)
      })
      expect(useEditorStore.getState().borrowedTool).toBe('pen')
      expect(useEditorStore.getState().tool).toBe('select')
      act(() => {
        window.dispatchEvent(new KeyboardEvent('keyup', { key: 'p' }))
      })
      expect(useEditorStore.getState().borrowedTool).toBeNull()
      expect(useEditorStore.getState().tool).toBe('select')
    } finally {
      vi.useRealTimers()
    }
  })

  it('keeps ⇧F on the ellipse even when Shift is released first', () => {
    vi.useFakeTimers()
    render(<Host />)
    try {
      act(() => {
        window.dispatchEvent(
          new KeyboardEvent('keydown', { key: 'F', shiftKey: true }),
        )
      })
      act(() => {
        window.dispatchEvent(
          new KeyboardEvent('keyup', { key: 'f', shiftKey: false }),
        )
      })
      expect(useEditorStore.getState().tool).toBe('ellipse')
    } finally {
      vi.useRealTimers()
    }
  })

  it('leaves a modified key to the command layer', () => {
    vi.useFakeTimers()
    render(<Host />)
    try {
      act(() => {
        window.dispatchEvent(
          new KeyboardEvent('keydown', { key: 'z', ctrlKey: true }),
        )
        window.dispatchEvent(
          new KeyboardEvent('keyup', { key: 'z', ctrlKey: true }),
        )
      })
      expect(useEditorStore.getState().tool).toBe('select')
    } finally {
      vi.useRealTimers()
    }
  })

  it('leaves tool keys inert while a modal is open', () => {
    vi.useFakeTimers()
    render(<Host />)
    try {
      useUiStore.getState().openModal()
      act(() => {
        window.dispatchEvent(new KeyboardEvent('keydown', { key: 'p' }))
        vi.advanceTimersByTime(HOLD_BORROW_MS)
      })
      expect(useEditorStore.getState().tool).toBe('select')
      expect(useEditorStore.getState().borrowedTool).toBeNull()
      act(() => {
        window.dispatchEvent(new KeyboardEvent('keyup', { key: 'p' }))
      })
      expect(useEditorStore.getState().tool).toBe('select')
    } finally {
      vi.useRealTimers()
    }
  })
})
