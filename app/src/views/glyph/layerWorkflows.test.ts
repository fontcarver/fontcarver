import { describe, expect, it } from 'vitest'
import { selectionOf } from '../../editor/selection'
import { makeContour, makeLayer, makePoint } from '../../test/fontFixtures'
import {
  clearBackgroundCommand,
  duplicateCustomLayerCommand,
  selectionToBackgroundCommand,
} from './layerWorkflows'

describe('per-glyph layer workflows', () => {
  it('duplicates all layer metadata into an independent custom layer', () => {
    const layer = makeLayer({ contours: [makeContour([makePoint(10, 20)])] })
    const command = duplicateCustomLayerCommand('a', layer, 'Alternate')
    if (!('AddLayer' in command)) throw new Error('expected layer creation')
    expect(command.AddLayer.layer).toMatchObject({
      kind: { Custom: 'Alternate' },
      name: 'Alternate',
      contours: layer.contours,
    })
    expect(command.AddLayer.layer.id).not.toBe(layer.id)
    expect(command.AddLayer.layer.contours).not.toBe(layer.contours)
  })

  it('copies each selected path once and selected components in stored order', () => {
    expect(
      selectionToBackgroundCommand(
        { doc: 1, glyph: 'a', layer: 'foreground' },
        'm1',
        selectionOf([
          { kind: 'node', contour: 2, point: 0 },
          { kind: 'handle', contour: 2, point: 1 },
          { kind: 'segment', contour: 0, segment: 1 },
          { kind: 'component', index: 3 },
          { kind: 'anchor', index: 0 },
        ]),
      ),
    ).toEqual({
      CopySelectionToBackground: {
        glyph: 'a',
        from: 'foreground',
        master: 'm1',
        contours: [0, 2],
        components: [3],
      },
    })
  })

  it('does not emit an empty command for an anchor-only selection', () => {
    expect(
      selectionToBackgroundCommand(
        { doc: 1, glyph: 'a', layer: 'foreground' },
        'm1',
        selectionOf([{ kind: 'anchor', index: 0 }]),
      ),
    ).toBeNull()
  })
})

describe('clearing a background layer', () => {
  it('empties contours and components in one batch, keeping the layer', () => {
    const background = makeLayer({
      id: 'bg',
      kind: { Background: 'm1' },
      contours: [
        makeContour([makePoint(0, 0)]),
        makeContour([makePoint(1, 1)]),
      ],
      components: [
        {
          base: 'acute',
          transform: [1, 0, 0, 1, 0, 0],
          anchor: null,
          auto_align: false,
          use_my_metrics: false,
          round_to_grid: false,
          smart: null,
          point_match: null,
        },
      ],
    })
    expect(clearBackgroundCommand('a', background)).toEqual({
      Batch: [
        { DeleteContours: { glyph: 'a', layer: 'bg', contours: [0, 1] } },
        { RemoveComponents: { glyph: 'a', layer: 'bg', indices: [0] } },
      ],
    })
  })

  it('sends one command when only one kind of content is there', () => {
    const contoursOnly = makeLayer({
      id: 'bg',
      kind: { Background: 'm1' },
      contours: [makeContour([makePoint(0, 0)])],
    })
    expect(clearBackgroundCommand('a', contoursOnly)).toEqual({
      DeleteContours: { glyph: 'a', layer: 'bg', contours: [0] },
    })
  })

  it('has nothing to do without a background, or with an empty one', () => {
    expect(clearBackgroundCommand('a', null)).toBeNull()
    expect(
      clearBackgroundCommand(
        'a',
        makeLayer({ id: 'bg', kind: { Background: 'm1' } }),
      ),
    ).toBeNull()
  })
})
