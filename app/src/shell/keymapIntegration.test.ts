import { afterEach, expect, it, vi } from 'vitest'
import { registerBuiltinCommands } from './builtinCommands'
import { resetCommands } from './commands'
import { keymapConflicts } from './keymap'
import { KEYMAP_PRESETS } from './keymapPresets'
import { keymapTargets, reservedKeymapTargets } from './keymapTargets'
import { commandForEvent } from './shortcuts'
import { metricShortcut } from '../views/text/textMetrics'
import { registerGlyphView } from '../views/glyph/glyphCommands'
import { initialUiState, useUiStore } from '../stores/uiStore'

afterEach(() => {
  resetCommands()
  useUiStore.setState(initialUiState())
})

it('keeps the integrated default and named preset keymaps conflict-free', () => {
  registerBuiltinCommands()
  const targets = [...keymapTargets(), ...reservedKeymapTargets()]
  expect(keymapConflicts(targets, {})).toEqual([])
  for (const preset of KEYMAP_PRESETS) {
    const known = new Set(targets.map((target) => target.id))
    expect(
      Object.keys(preset.overrides).filter((id) => !known.has(id)),
      preset.id,
    ).toEqual([])
    expect(keymapConflicts(targets, preset.overrides), preset.id).toEqual([])
  }
})

it.each([
  '[',
  'Shift+[',
  'Alt+[',
  'Alt+Shift+[',
  ']',
  'Shift+]',
  'Alt+]',
  'Alt+Shift+]',
])(
  'detects user bindings that conflict with the fixed %s spacing gesture',
  (shortcut) => {
    registerBuiltinCommands()
    const targets = [...keymapTargets(), ...reservedKeymapTargets()]
    expect(keymapConflicts(targets, { 'file.open': shortcut })).toEqual([
      { shortcut, ids: ['file.open', `gesture.spacing.${shortcut}`] },
    ])
  },
)

it.each([false, true])(
  'leaves Alt spacing chords to the canvas on macOS=%s',
  (mac) => {
    registerBuiltinCommands()
    for (const key of ['[', ']']) {
      for (const shiftKey of [false, true]) {
        const event = new KeyboardEvent('keydown', {
          key,
          altKey: true,
          shiftKey,
        })
        expect(commandForEvent(event, mac)).toBeUndefined()
        expect(
          metricShortcut(event, mac, { side: 'both', step: 2.5 })?.delta,
        ).toBe((key === '[' ? -1 : 1) * (shiftKey ? 1 : 250))
      }
    }
  },
)

it.each([false, true])(
  'dispatches Mod brackets only to glyph navigation on macOS=%s',
  (mac) => {
    registerBuiltinCommands()
    useUiStore.setState({
      tabs: [{ id: 'glyph', kind: 'glyph', doc: 1, gid: 7 }],
      activeTab: 'glyph',
    })
    const dispose = registerGlyphView({
      doc: 1,
      gid: 7,
      fit: vi.fn(),
      zoomBy: vi.fn(),
      step: vi.fn(),
    })
    try {
      for (const [key, command] of [
        ['[', 'glyph.previous'],
        [']', 'glyph.next'],
      ] as const) {
        const event = new KeyboardEvent('keydown', {
          key,
          ctrlKey: !mac,
          metaKey: mac,
        })
        expect(commandForEvent(event, mac)).toBe(command)
        expect(
          metricShortcut(event, mac, { side: 'both', step: 2.5 }),
        ).toBeNull()
      }
    } finally {
      dispose()
    }
  },
)
