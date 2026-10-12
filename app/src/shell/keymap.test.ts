import { afterEach, describe, expect, it, vi } from 'vitest'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { readKeymap, useKeymapStore } from '../stores/keymapStore'
import { registerCommands, resetCommands } from './commands'
import { keymapConflicts, shortcutFor } from './keymap'
import { parseKeymapDocument, serializeKeymap } from './keymapDocument'
import { commandForEvent, installShortcuts, parseShortcut } from './shortcuts'

afterEach(() => {
  resetCommands()
  useKeymapStore.setState({ overrides: {}, open: false })
  useUiStore.setState(initialUiState())
  localStorage.clear()
  document.body.innerHTML = ''
  vi.restoreAllMocks()
})

const targets = [
  { id: 'file.open', title: 'open', shortcut: 'Mod+O', scope: 'global' },
  { id: 'edit.selectAll', title: 'glyphs', shortcut: 'Mod+A', scope: 'font' },
  {
    id: 'edit.selectAllPoints',
    title: 'points',
    shortcut: 'Mod+A',
    scope: 'glyph',
  },
]

describe('keymap validation', () => {
  it('rejects ambiguous and unsupported notation', () => {
    for (const value of [
      'A+B',
      'Mod+Mod+A',
      'Mod+',
      'Mod++A',
      'Hyper+A',
      'Tab',
    ]) {
      if (value !== 'Tab') expect(parseShortcut(value)).toBeNull()
    }
    expect(() =>
      parseKeymapDocument('{"version":1,"bindings":{"x":"Tab"}}'),
    ).toThrow()
    expect(() => parseKeymapDocument('{"version":2,"bindings":{}}')).toThrow()
    expect(() =>
      parseKeymapDocument('{"version":1,"bindings":{"x":42}}'),
    ).toThrow()
  })
  it('round-trips portable shortcuts and explicit unbindings', () => {
    const overrides = { 'file.open': 'Mod+Alt+O', 'file.save': null }
    expect(parseKeymapDocument(serializeKeymap(overrides))).toEqual(overrides)
    expect(shortcutFor('file.open', 'Mod+O', overrides)).toBe('Mod+Alt+O')
    expect(shortcutFor('file.save', 'Mod+S', overrides)).toBeNull()
  })
  it('detects overlapping scopes while preserving mutually exclusive defaults', () => {
    expect(keymapConflicts(targets, {})).toEqual([])
    expect(keymapConflicts(targets, { 'file.open': 'Ctrl+A' })).toEqual([
      {
        shortcut: 'Mod+A',
        ids: ['file.open', 'edit.selectAll', 'edit.selectAllPoints'],
      },
    ])
  })
})

describe('live bindings and persistence', () => {
  it('persists and restores a custom binding without mutating on failed storage', () => {
    expect(useKeymapStore.getState().save({ 'file.open': 'Mod+K' })).toBe(true)
    expect(readKeymap()).toEqual({ 'file.open': 'Mod+K' })
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('quota')
    })
    expect(useKeymapStore.getState().save({ 'file.open': null })).toBe(false)
    expect(useKeymapStore.getState().overrides['file.open']).toBe('Mod+K')
  })
  it('keeps a saved user command ahead of newly introduced defaults', () => {
    registerCommands([
      {
        id: 'palette.quickOpen',
        title: 'quick',
        shortcut: 'Mod+K',
        run: vi.fn(),
      },
      { id: 'file.open', title: 'open', shortcut: 'Mod+O', run: vi.fn() },
    ])
    useKeymapStore.getState().save({ 'file.open': 'Mod+K' })
    expect(
      commandForEvent(
        new KeyboardEvent('keydown', { key: 'k', ctrlKey: true }),
        false,
      ),
    ).toBe('file.open')
  })

  it('dispatches current overrides and explicit unbindings immediately', () => {
    registerCommands([
      { id: 'file.open', title: 'x', shortcut: 'Mod+O', run: vi.fn() },
    ])
    useKeymapStore.getState().save({ 'file.open': 'Mod+K' })
    expect(
      commandForEvent(
        new KeyboardEvent('keydown', { key: 'o', ctrlKey: true }),
        false,
      ),
    ).toBeUndefined()
    expect(
      commandForEvent(
        new KeyboardEvent('keydown', { key: 'k', ctrlKey: true }),
        false,
      ),
    ).toBe('file.open')
    useKeymapStore.getState().save({ 'file.open': null })
    expect(
      commandForEvent(
        new KeyboardEvent('keydown', { key: 'k', ctrlKey: true }),
        false,
      ),
    ).toBeUndefined()
  })
  it('preserves contenteditable descendants, IME composition and native buttons', () => {
    const run = vi.fn()
    registerCommands([
      {
        id: 'file.open',
        title: 'x',
        shortcut: 'Enter',
        allowInInput: true,
        run,
      },
    ])
    const stop = installShortcuts()
    const editable = document.createElement('div')
    editable.setAttribute('contenteditable', 'true')
    editable.innerHTML = '<span>writing</span>'
    const button = document.createElement('button')
    document.body.append(editable, button)
    for (const target of [editable.firstChild!, button])
      target.dispatchEvent(
        new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }),
      )
    window.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'Enter', isComposing: true }),
    )
    expect(run).not.toHaveBeenCalled()
    stop()
  })
  it('allows context help through a modal while blocking document commands', () => {
    const help = vi.fn(),
      edit = vi.fn()
    registerCommands([
      {
        id: 'help.context',
        title: 'help',
        shortcut: 'F1',
        allowInInput: true,
        allowInModal: true,
        run: help,
      },
      { id: 'edit.test', title: 'edit', shortcut: 'Mod+E', run: edit },
    ])
    useUiStore.getState().openModal()
    const stop = installShortcuts()
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'F1' }))
    window.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'e', ctrlKey: true }),
    )
    expect(help).toHaveBeenCalledOnce()
    expect(edit).not.toHaveBeenCalled()
    stop()
  })
})
