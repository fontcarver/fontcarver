import { afterEach, beforeEach, expect, it } from 'vitest'

import { setTransport } from '../api'
import { initialDocumentState, useDocumentStore } from '../stores/documentStore'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { TOOLS } from '../editor/tools/registry'

import { registerBuiltinCommands } from './builtinCommands'
import { listCommands, resetCommands } from './commands'
import { keymapTargets, reservedKeymapTargets } from './keymapTargets'
import {
  commandForEvent,
  isMacPlatform,
  matchesShortcut,
  parseShortcut,
} from './shortcuts'
import { registerWorkspaceTabCommands } from './workspaceTabCommands'

/**
 * Row 9: one keymap for macOS, Windows and Linux, differing only in whether
 * `Mod` means Command or Control.
 *
 * The parity claim is about the definition, not about three test runs: there is
 * a single table, every binding in it is written in the platform-independent
 * `Mod` notation, and the dispatcher resolves that notation from one boolean.
 * These tests hold the table to that shape, so a new platform-specific binding
 * cannot be added without failing here. Running the shell on macOS and Windows
 * is separate work and stays recorded as pending native execution.
 */
beforeEach(() => {
  localStorage.clear()
  resetCommands()
  useUiStore.setState(initialUiState())
  useSessionStore.setState(initialSessionState())
  useDocumentStore.setState(initialDocumentState())
  registerBuiltinCommands()
  registerWorkspaceTabCommands()
})

afterEach(() => {
  resetCommands()
  setTransport(undefined)
})

function platform(value: string): void {
  Object.defineProperty(globalThis.navigator, 'platform', {
    configurable: true,
    value,
  })
}

it('defines every action once, with one platform-independent binding', () => {
  const ids = listCommands().map((command) => command.id)
  expect(new Set(ids).size).toBe(ids.length)

  const targets = [...keymapTargets(), ...reservedKeymapTargets()]
  expect(targets.length).toBeGreaterThan(ids.length - 5)
  for (const command of listCommands()) {
    if (command.shortcut === undefined) continue
    // `Cmd`, `Command`, `Meta`, `Win` and `Super` would each be a binding that
    // only works on one platform; the abstraction exists so none is needed.
    expect(command.shortcut).not.toMatch(/\b(Cmd|Command|Meta|Win|Super)\b/)
    expect(parseShortcut(command.shortcut), command.id).not.toBeNull()
  }
  for (const tool of TOOLS)
    expect(`${tool.shift ? 'Shift+' : ''}${tool.key.toUpperCase()}`).toMatch(
      /^(Shift\+)?[A-Z0-9]$/,
    )
})

it('reads Mod as Command on macOS and Control everywhere else', () => {
  platform('MacIntel')
  expect(isMacPlatform()).toBe(true)
  platform('Win32')
  expect(isMacPlatform()).toBe(false)
  platform('Linux x86_64')
  expect(isMacPlatform()).toBe(false)

  const parsed = parseShortcut('Mod+Alt+2')
  expect(parsed).not.toBeNull()
  const event = (init: Partial<KeyboardEvent>) =>
    new KeyboardEvent('keydown', init)

  const command = event({ key: '2', metaKey: true, altKey: true })
  const control = event({ key: '2', ctrlKey: true, altKey: true })
  expect(matchesShortcut(command, parsed!, true)).toBe(true)
  expect(matchesShortcut(control, parsed!, true)).toBe(false)
  expect(matchesShortcut(control, parsed!, false)).toBe(true)
  expect(matchesShortcut(command, parsed!, false)).toBe(false)
})

it('dispatches the same actions on all three platforms modulo Cmd/Ctrl', () => {
  useSessionStore.setState({
    docs: [
      {
        doc: 1,
        path: '/fonts/Family.fcf',
        dirty: false,
        family_name: 'Family',
        style_name: 'Regular',
        units_per_em: 1000,
        glyph_count: 1,
        masters: [],
        axes: [],
        undo_label: null,
        redo_label: null,
        format: 'fcf',
        warnings: [],
      },
    ],
  })
  useUiStore.getState().openTab({ kind: 'font', doc: 1 })
  useUiStore.getState().openTab({ kind: 'glyph', doc: 1, gid: 3 })

  const bound = listCommands().filter(
    (command) => command.shortcut !== undefined,
  )
  expect(bound.length).toBeGreaterThan(20)

  for (const command of bound) {
    const parsed = parseShortcut(command.shortcut!)
    if (!parsed) continue
    const init: Partial<KeyboardEvent> = {
      key: parsed.key,
      shiftKey: parsed.shift,
      altKey: parsed.alt,
    }
    const mac = new KeyboardEvent('keydown', {
      ...init,
      metaKey: parsed.mod,
      ctrlKey: Boolean(parsed.control),
    })
    const pc = new KeyboardEvent('keydown', {
      ...init,
      ctrlKey: parsed.mod || Boolean(parsed.control),
    })
    // A binding with no Mod is identical on both sides, and the resolved
    // command must be the same action either way.
    expect(commandForEvent(mac, true), command.id).toBe(
      commandForEvent(pc, false),
    )
  }
})
