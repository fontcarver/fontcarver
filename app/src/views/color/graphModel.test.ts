import { expect, it } from 'vitest'
import { addGradient, parseGraph, graphPaletteIndices } from './graphModel'
import { emptyTable, removeEntry } from './colorModel'

it('validates structural graph data before palette and preview code use it', () => {
  const graph = addGradient(null, 'A', 2)
  expect(parseGraph(JSON.stringify(graph))).toEqual(graph)
  for (const value of [
    null,
    [],
    {},
    { glyphs: {}, paints: [null] },
    { ...graph, clips: { A: [1] } },
    { ...graph, paints: [{ kind: 'Solid', alpha: 1 }] },
  ]) {
    expect(() => parseGraph(JSON.stringify(value))).toThrow(
      'color.graphInvalid',
    )
  }
  expect(() => parseGraph('{')).toThrow('color.graphInvalid')
})
it('blocks deletion of graph colors and shifts every gradient and solid reference', () => {
  const table = emptyTable()
  table.palettes[0].colors = Array.from({ length: 3 }, () => ({
    red: 0,
    green: 0,
    blue: 0,
    alpha: 255,
  }))
  table.graph = {
    glyphs: { A: 0 },
    clips: {},
    paints: [
      { kind: 'Solid', palette_index: 2, alpha: 1 },
      {
        kind: 'Sweep',
        center: [0, 0],
        angles: [0, 360],
        color_line: {
          extend: 0,
          stops: [
            { offset: 0, palette_index: 2, alpha: 1 },
            { offset: 1, palette_index: null, alpha: 1 },
          ],
        },
      },
    ],
  }
  expect(removeEntry(table, 2)).toBe(table)
  const next = removeEntry(table, 0)
  expect(next.palettes[0].colors).toHaveLength(2)
  expect(graphPaletteIndices(next.graph)).toEqual([1, 1, null])
  expect(graphPaletteIndices(table.graph)).toEqual([2, 2, null])
})
