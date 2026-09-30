import { afterEach, expect, it } from 'vitest'
import { registerBuiltinCommands } from './builtinCommands'
import { resetCommands } from './commands'
import { keymapConflicts } from './keymap'
import { KEYMAP_PRESETS } from './keymapPresets'
import { keymapTargets, reservedKeymapTargets } from './keymapTargets'

afterEach(resetCommands)

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
