import { describe, expect, it } from 'vitest'

import { TOOLS } from '../editor/tools/registry'

import { keymapConflicts } from './keymap'
import { parseKeymapDocument, serializeKeymap } from './keymapDocument'
import { KEYMAP_PRESETS } from './keymapPresets'
import { toolShortcut } from './toolShortcuts'

describe('compatibility keymap presets', () => {
  it('ships distinct Glyphs, FontLab and RoboFont subsets with portable documents', () => {
    expect(KEYMAP_PRESETS.map((preset) => preset.id)).toEqual([
      'glyphs',
      'fontlab',
      'robofont',
    ])
    for (const preset of KEYMAP_PRESETS) {
      expect(parseKeymapDocument(serializeKeymap(preset.overrides))).toEqual(
        preset.overrides,
      )
    }
  })

  it('keeps all tool assignments unique after merging each subset with defaults', () => {
    const tools = TOOLS.map((tool) => ({
      id: `tool.${tool.id}`,
      title: tool.labelKey,
      shortcut: toolShortcut(tool.id, {}) ?? undefined,
      scope: 'glyph',
    }))
    for (const preset of KEYMAP_PRESETS) {
      expect(keymapConflicts(tools, preset.overrides), preset.id).toEqual([])
    }
  })

  it('uses FontLab contour selection and RoboFont numbered tools', () => {
    const fontlab = KEYMAP_PRESETS.find((preset) => preset.id === 'fontlab')
    const robofont = KEYMAP_PRESETS.find((preset) => preset.id === 'robofont')
    expect(fontlab?.overrides['tool.select']).toBe('A')
    expect(fontlab?.overrides['tool.knife']).toBe('J')
    expect(robofont?.overrides['tool.select']).toBe('1')
    expect(robofont?.overrides['tool.measure']).toBe('4')
    expect(robofont?.overrides['file.export']).toBe('Mod+Alt+G')
  })
})
