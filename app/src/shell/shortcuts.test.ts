import { afterEach, describe, expect, it, vi } from 'vitest'

import { initialUiState, useUiStore } from '../stores/uiStore'

import { registerCommand, resetCommands } from './commands'
import {
  commandForEvent,
  formatShortcut,
  installShortcuts,
  isEditableTarget,
  parseShortcut,
} from './shortcuts'

afterEach(() => {
  resetCommands()
  document.body.innerHTML = ''
})

describe('parseShortcut', () => {
  it('splits modifiers from the key', () => {
    expect(parseShortcut('Mod+Shift+P')).toEqual({
      mod: true,
      shift: true,
      alt: false,
      key: 'p',
    })
    expect(parseShortcut('Mod+=')).toEqual({
      mod: true,
      shift: false,
      alt: false,
      key: '=',
    })
    expect(parseShortcut('')).toBeNull()
  })
})

describe('formatShortcut', () => {
  it('renders platform-appropriate labels', () => {
    expect(formatShortcut('Mod+Shift+P', true)).toBe('⌘⇧P')
    expect(formatShortcut('Mod+Shift+P', false)).toBe('Ctrl+Shift+P')
    expect(formatShortcut('Mod+Alt+ArrowRight', false)).toBe('Ctrl+Alt+→')
  })
})

describe('installShortcuts', () => {
  it('runs the bound command and prevents the browser default', () => {
    const run = vi.fn()
    registerCommand({
      id: 'test.palette',
      title: 'x',
      shortcut: 'Mod+Shift+P',
      run,
    })
    const stop = installShortcuts(window)

    const event = new KeyboardEvent('keydown', {
      key: 'p',
      code: 'KeyP',
      ctrlKey: true,
      shiftKey: true,
      bubbles: true,
      cancelable: true,
    })
    document.body.dispatchEvent(event)

    expect(run).toHaveBeenCalledOnce()
    expect(event.defaultPrevented).toBe(true)
    stop()
  })

  it('ignores shortcuts typed into a text field unless the command opts in', () => {
    const blocked = vi.fn()
    const allowed = vi.fn()
    registerCommand({
      id: 'test.open',
      title: 'x',
      shortcut: 'Mod+O',
      run: blocked,
    })
    registerCommand({
      id: 'test.always',
      title: 'x',
      shortcut: 'Mod+K',
      allowInInput: true,
      run: allowed,
    })
    const stop = installShortcuts(window)

    const input = document.createElement('input')
    document.body.append(input)
    expect(isEditableTarget(input)).toBe(true)

    input.dispatchEvent(
      new KeyboardEvent('keydown', {
        key: 'o',
        code: 'KeyO',
        ctrlKey: true,
        bubbles: true,
        cancelable: true,
      }),
    )
    input.dispatchEvent(
      new KeyboardEvent('keydown', {
        key: 'k',
        code: 'KeyK',
        ctrlKey: true,
        bubbles: true,
        cancelable: true,
      }),
    )

    expect(blocked).not.toHaveBeenCalled()
    expect(allowed).toHaveBeenCalledOnce()
    stop()
  })

  it('stops dispatching once disposed', () => {
    const run = vi.fn()
    registerCommand({ id: 'test.save', title: 'x', shortcut: 'Mod+S', run })
    const stop = installShortcuts(window)
    stop()

    document.body.dispatchEvent(
      new KeyboardEvent('keydown', {
        key: 's',
        code: 'KeyS',
        ctrlKey: true,
        bubbles: true,
        cancelable: true,
      }),
    )

    expect(run).not.toHaveBeenCalled()
  })
})

describe('commandForEvent', () => {
  it('resolves the command bound to an event', () => {
    registerCommand({
      id: 'test.undo',
      title: 'x',
      shortcut: 'Mod+Z',
      run: vi.fn(),
    })

    const event = new KeyboardEvent('keydown', {
      key: 'z',
      code: 'KeyZ',
      ctrlKey: true,
    })

    expect(commandForEvent(event, false)).toBe('test.undo')
  })
})

describe('shortcuts under a modal dialog', () => {
  afterEach(() => {
    useUiStore.setState(initialUiState())
  })

  function bind(run: () => void): () => void {
    registerCommand({ id: 'test.close', title: 'x', shortcut: 'Mod+W', run })
    return installShortcuts(window)
  }

  function pressModW(): void {
    window.dispatchEvent(
      new KeyboardEvent('keydown', {
        key: 'w',
        code: 'KeyW',
        ctrlKey: true,
        bubbles: true,
        cancelable: true,
      }),
    )
  }

  it('leaves the keyboard to a dialog while one is on screen', () => {
    const run = vi.fn()
    const stop = bind(run)

    useUiStore.getState().openModal()
    pressModW()
    expect(run).not.toHaveBeenCalled()

    useUiStore.getState().closeModal()
    pressModW()
    expect(run).toHaveBeenCalledTimes(1)

    stop()
  })

  it('never swallows Escape, which the dialog itself handles', () => {
    const run = vi.fn()
    registerCommand({
      id: 'test.escape',
      title: 'x',
      shortcut: 'Escape',
      run,
    })
    const stop = installShortcuts(window)
    useUiStore.getState().openModal()

    window.dispatchEvent(
      new KeyboardEvent('keydown', {
        key: 'Escape',
        code: 'Escape',
        bubbles: true,
        cancelable: true,
      }),
    )

    expect(run).toHaveBeenCalledTimes(1)
    stop()
  })
})
