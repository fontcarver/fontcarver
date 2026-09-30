import type { KeymapOverrides } from './keymap'

/** A documented subset of another editor's shortcuts. */
export interface KeymapPreset {
  id: 'glyphs' | 'fontlab' | 'robofont'
  nameKey: string
  overrides: KeymapOverrides
}

/**
 * Compatibility subsets, applied over FontCarver defaults. Official sources,
 * supported mappings and intentional differences: docs/keymap-presets.md.
 */
export const KEYMAP_PRESETS: readonly KeymapPreset[] = [
  {
    id: 'glyphs',
    nameKey: 'shortcuts.preset.glyphs',
    overrides: {
      'tool.select': 'V',
      'tool.pen': 'P',
      'tool.pencil': 'B',
      'tool.knife': 'E',
      'tool.rectangle': 'F',
      'tool.ellipse': 'Shift+F',
      'tool.measure': 'L',
      'tool.rotate': 'R',
      'tool.scale': 'S',
      'tool.zoom': 'Z',
      'tool.hand': 'H',
      'tool.text': 'T',
      'file.new': 'Mod+N',
      'file.save': 'Mod+S',
      'file.saveAs': 'Mod+Shift+S',
    },
  },
  {
    id: 'fontlab',
    nameKey: 'shortcuts.preset.fontlab',
    overrides: {
      'tool.select': 'A',
      // FontCarver's annotation tool has no counterpart in this subset.
      'tool.annotation': null,
      'tool.pen': 'P',
      'tool.pencil': 'N',
      'tool.knife': 'J',
      'tool.rectangle': 'I',
      'tool.ellipse': 'O',
      'tool.rotate': 'R',
      'tool.scale': 'S',
      'tool.text': 'T',
      'file.new': 'Mod+N',
      'file.open': 'Mod+O',
      'file.save': 'Mod+S',
      'file.saveAs': 'Mod+Shift+S',
      'file.export': 'Mod+E',
      'edit.undo': 'Mod+Z',
      'edit.redo': 'Mod+Shift+Z',
    },
  },
  {
    id: 'robofont',
    nameKey: 'shortcuts.preset.robofont',
    overrides: {
      'tool.select': '1',
      'tool.pen': '2',
      'tool.knife': '3',
      'tool.measure': '4',
      'file.new': 'Mod+N',
      'file.open': 'Mod+O',
      'file.save': 'Mod+S',
      'file.saveAs': 'Mod+Shift+S',
      'file.close': 'Mod+W',
      'file.export': 'Mod+Alt+G',
      'edit.undo': 'Mod+Z',
      'edit.redo': 'Mod+Shift+Z',
    },
  },
]
