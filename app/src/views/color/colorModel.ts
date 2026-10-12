import { graphPaletteIndices, shiftGraphPalette } from './graphModel'
import type { ColorTable, PaletteColor } from '../../api'

export function emptyTable(): ColorTable {
  return {
    palette_version: 0,
    palettes: [
      {
        colors: [{ red: 42, green: 112, blue: 220, alpha: 255 }],
        flags: 0,
        label: null,
      },
    ],
    entry_labels: [],
    glyphs: {},
  }
}

export function formatColor(c: PaletteColor): string {
  return (
    '#' +
    [c.red, c.green, c.blue, c.alpha]
      .map((v) => v.toString(16).padStart(2, '0'))
      .join('')
      .toUpperCase()
  )
}

export function parseColor(text: string): PaletteColor | undefined {
  if (!/^#[\da-f]{6}([\da-f]{2})?$/i.test(text.trim())) return undefined
  const value = text.trim().slice(1).padEnd(8, 'F')
  const channel = (start: number) => parseInt(value.slice(start, start + 2), 16)
  return {
    red: channel(0),
    green: channel(2),
    blue: channel(4),
    alpha: channel(6),
  }
}

/** Remove an unused shared entry and update every higher layer index. */
export function removeEntry(table: ColorTable, index: number): ColorTable {
  if (
    table.palettes[0].colors.length < 2 ||
    graphPaletteIndices(table.graph).includes(index) ||
    Object.values(table.glyphs).some((layers) =>
      layers.some((l) => l.palette_index === index),
    )
  )
    return table
  return {
    ...table,
    graph: table.graph ? shiftGraphPalette(table.graph, index) : table.graph,
    palettes: table.palettes.map((p) => ({
      ...p,
      colors: p.colors.filter((_, i) => i !== index),
    })),
    entry_labels: table.entry_labels.filter((_, i) => i !== index),
    glyphs: Object.fromEntries(
      Object.entries(table.glyphs).map(([name, layers]) => [
        name,
        layers.map((l) => ({
          ...l,
          palette_index:
            l.palette_index !== null && l.palette_index > index
              ? l.palette_index - 1
              : l.palette_index,
        })),
      ]),
    ),
  }
}
