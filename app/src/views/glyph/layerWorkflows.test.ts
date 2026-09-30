import { describe, expect, it } from 'vitest'
import { selectionOf } from '../../editor/selection'
import { makeContour, makeLayer, makePoint } from '../../test/fontFixtures'
import {
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
