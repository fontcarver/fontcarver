import { describe, expect, it } from 'vitest'

import { makeContour, makeLayer, makePoint } from '../../../test/fontFixtures'
import {
  magneticLinkCommand,
  setServantCommand,
  stemGuidesCommand,
} from './constructionCommands'
import { detectStraightStems } from './constructionGeometry'

const target = { doc: 1, glyph: 'H', layer: 'layer-1' }

describe('construction commands', () => {
  it('creates X interpolation only when the independent boundary span is nonzero', () => {
    const layer = makeLayer({
      contours: [
        makeContour(
          [makePoint(0, 0, 'Move'), makePoint(50, 10), makePoint(100, 0)],
          false,
        ),
      ],
    })
    expect(
      setServantCommand(target, layer, { contour: 0, point: 1 }, 'X'),
    ).toEqual({
      NodeLinks: {
        SetServant: {
          glyph: 'H',
          layer: 'layer-1',
          node: { contour: 0, point: 1 },
          first: { contour: 0, point: 0 },
          second: { contour: 0, point: 2 },
          axis: 'X',
        },
      },
    })
    expect(
      setServantCommand(target, layer, { contour: 0, point: 1 }, 'Y'),
    ).toBeNull()
  })

  it('keeps projection and persistent linking in a single undo transaction', () => {
    const command = magneticLinkCommand(
      target,
      2,
      [{ contour: 0, point: 1 }],
      true,
    )
    expect(command).toMatchObject({
      Batch: [
        { NodeLinks: { ProjectToGuide: { guide_index: 2 } } },
        { NodeLinks: { LinkToGuide: { guide_index: 2 } } },
      ],
    })
  })

  it('links both detected edges to a single centerline guide preserving width', () => {
    const layer = makeLayer({
      contours: [
        makeContour([
          makePoint(0, 0),
          makePoint(60, 0),
          makePoint(60, 700),
          makePoint(0, 700),
        ]),
      ],
    })
    const command = stemGuidesCommand(
      target,
      layer,
      detectStraightStems(layer.contours, 200),
      (width) => `Stem ${width}`,
    )
    expect(command).toMatchObject({
      Batch: [
        { AddGuide: { guide: { x: 30, angle: 90, name: 'Stem 60' } } },
        {
          NodeLinks: {
            LinkToGuide: {
              guide_index: 0,
              points: expect.arrayContaining([
                { contour: 0, point: 0 },
                { contour: 0, point: 1 },
                { contour: 0, point: 2 },
                { contour: 0, point: 3 },
              ]),
            },
          },
        },
      ],
    })
  })
  it('shares one center guide across separated stems without dropping node links', () => {
    const rectangle = (x: number, y: number, width: number) =>
      makeContour([
        makePoint(x, y),
        makePoint(x + width, y),
        makePoint(x + width, y + 300),
        makePoint(x, y + 300),
      ])
    const layer = makeLayer({
      contours: [rectangle(0, 0, 60), rectangle(10, 400, 40)],
    })
    const command = stemGuidesCommand(
      target,
      layer,
      detectStraightStems(layer.contours, 100),
      String,
    )
    expect(command && 'Batch' in command ? command.Batch.length : 0).toBe(2)
    const link = command && 'Batch' in command ? command.Batch[1] : null
    expect(
      link && 'NodeLinks' in link && 'LinkToGuide' in link.NodeLinks
        ? link.NodeLinks.LinkToGuide.points.length
        : 0,
    ).toBe(8)
  })
})
