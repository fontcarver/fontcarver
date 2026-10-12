import { describe, expect, it } from 'vitest'

import type { Component } from '../../../api'
import type { LayerTarget } from '../../../editor/pointCommands'

import {
  addComponentCommand,
  addGuideCommand,
  addLayerCommand,
  copyLayerCommand,
  decomposeComponentsCommand,
  duplicateName,
  hasMetricsKeys,
  metricsKeysWith,
  newLayerId,
  removeGuideCommand,
  reorderComponentsCommand,
  reordered,
  setGlyphMetaCommand,
  suggestAnchorName,
} from './panelCommands'

const target: LayerTarget = { doc: 1, glyph: 'a', layer: 'layer-1' }

function component(base: string): Component {
  return {
    base,
    transform: [1, 0, 0, 1, 0, 0],
    anchor: null,
    auto_align: false,
    locked: false,
    use_my_metrics: false,
    round_to_grid: false,
    smart: null,
    point_match: null,
  }
}

describe('glyph meta', () => {
  it('leaves every field the caller did not name alone', () => {
    expect(setGlyphMetaCommand('a', { export: false })).toEqual({
      SetGlyphMeta: {
        glyph: 'a',
        export: false,
        tags: null,
        note: null,
      },
    })
  })

  it('distinguishes clearing the colour label from leaving it', () => {
    expect(setGlyphMetaCommand('a', { colorLabel: null })).toEqual({
      SetGlyphMeta: {
        glyph: 'a',
        export: null,
        color_label: null,
        tags: null,
        note: null,
      },
    })
    expect(setGlyphMetaCommand('a', { colorLabel: 3 })).toMatchObject({
      SetGlyphMeta: { color_label: 3 },
    })
    // No key at all: the command carries no `color_label` field.
    expect(
      Object.hasOwn(
        (setGlyphMetaCommand('a', { note: 'x' }) as { SetGlyphMeta: object })
          .SetGlyphMeta,
        'color_label',
      ),
    ).toBe(false)
  })

  it('names a duplicate that no existing glyph uses', () => {
    expect(duplicateName('A', new Set())).toBe('A.copy')
    expect(duplicateName('A', new Set(['A.copy']))).toBe('A.copy2')
    expect(duplicateName('A', new Set(['A.copy', 'A.copy2']))).toBe('A.copy3')
  })
})

describe('metrics keys', () => {
  const empty = {
    left: null,
    right: null,
    width: null,
    vertical_origin: null,
    vertical_width: null,
  }

  it('normalises an emptied field back to null', () => {
    const set = metricsKeysWith(empty, 'left', ' =H ')
    expect(set.left).toBe('=H')
    expect(metricsKeysWith(set, 'left', '   ').left).toBeNull()
  })

  it('reports whether any horizontal formula is set', () => {
    expect(hasMetricsKeys(empty)).toBe(false)
    expect(hasMetricsKeys({ ...empty, right: '=|n' })).toBe(true)
    // A vertical formula alone is not a horizontal one.
    expect(hasMetricsKeys({ ...empty, vertical_width: '=500' })).toBe(false)
  })
})

describe('layers', () => {
  it('adds an empty background layer carrying the master width', () => {
    const command = addLayerCommand(
      'a',
      { Background: 'm1' },
      'Background',
      600,
    )
    expect(command).toMatchObject({
      AddLayer: {
        glyph: 'a',
        layer: {
          kind: { Background: 'm1' },
          name: 'Background',
          width: 600,
          contours: [],
        },
      },
    })
  })

  it('gives every new layer its own identifier', () => {
    expect(newLayerId()).not.toBe(newLayerId())
  })

  it('copies the edited layer into a timestamped backup', () => {
    expect(
      copyLayerCommand('a', 'layer-1', { Backup: { of: 'm1', time: 42 } }),
    ).toEqual({
      CopyLayer: {
        glyph: 'a',
        from: 'layer-1',
        to: { Backup: { of: 'm1', time: 42 } },
      },
    })
  })
})

describe('components', () => {
  it('adds a component at the origin with no options set', () => {
    expect(addComponentCommand(target, 'acute')).toEqual({
      AddComponent: {
        glyph: 'a',
        layer: 'layer-1',
        component: component('acute'),
      },
    })
  })

  it('decomposes only the listed components', () => {
    expect(decomposeComponentsCommand(target, [1])).toEqual({
      DecomposeComponents: { glyph: 'a', layer: 'layer-1', indices: [1] },
    })
  })

  it('moves an item without disturbing the rest of the list', () => {
    expect(reordered(['a', 'b', 'c'], 2, 0)).toEqual(['c', 'a', 'b'])
    expect(reordered(['a', 'b', 'c'], 0, 2)).toEqual(['b', 'c', 'a'])
  })

  it('reorders with one SetComponent per row that actually moved', () => {
    const list = [component('a'), component('b'), component('c')]
    const command = reorderComponentsCommand(target, list, 0, 1)

    // Rows 0 and 1 swap; row 2 is untouched and gets no write.
    expect(command).toEqual({
      Batch: [
        {
          SetComponent: {
            glyph: 'a',
            layer: 'layer-1',
            index: 0,
            component: list[1],
          },
        },
        {
          SetComponent: {
            glyph: 'a',
            layer: 'layer-1',
            index: 1,
            component: list[0],
          },
        },
      ],
    })
  })

  it('sends nothing for a reorder that changes nothing', () => {
    const list = [component('a'), component('b')]
    expect(reorderComponentsCommand(target, list, 1, 1)).toBeNull()
  })
})

describe('anchors', () => {
  it('suggests the first conventional name the layer lacks', () => {
    expect(suggestAnchorName([])).toBe('top')
    expect(suggestAnchorName([{ name: 'top', x: 0, y: 0 }])).toBe('bottom')
  })
})

describe('guides', () => {
  it('scopes a layer guide to the glyph and a global one to the master', () => {
    const guide = {
      x: 0,
      y: 500,
      angle: 0,
      name: null,
      locked: false,
      color: null,
      anchor: 'Origin' as const,
    }

    expect(addGuideCommand(target, 'layer', 'm1', guide)).toEqual({
      AddGuide: { glyph: 'a', layer: 'layer-1', master: null, guide },
    })
    expect(addGuideCommand(target, 'master', 'm1', guide)).toEqual({
      AddGuide: { glyph: null, layer: null, master: 'm1', guide },
    })
    expect(removeGuideCommand(target, 'master', 'm1', 2)).toEqual({
      RemoveGuide: { glyph: null, layer: null, master: 'm1', index: 2 },
    })
  })
})
