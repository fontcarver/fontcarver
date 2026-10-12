import { describe, expect, it } from 'vitest'

import {
  makeContour as contour,
  makePoint as point,
} from '../test/fontFixtures'
import { buildEditOutline } from './outlineGeometry'
import {
  editorMetadata,
  editableOutline,
  selectedContours,
} from './editorMetadata'
import { selectionOf } from './selection'

const annotation = {
  id: 'note',
  kind: 'Text',
  x: 20,
  y: 50,
  end_x: 20,
  end_y: 50,
  text: 'Check stem',
}

describe('layer editor metadata', () => {
  it('reads only valid annotations in the application namespace', () => {
    expect(editorMetadata({ annotations: [annotation] }).annotations).toEqual(
      [],
    )
    const data = editorMetadata({
      'com.fontcarver.editor': {
        annotations: [
          annotation,
          { ...annotation, x: NaN },
          { ...annotation, kind: 'script' },
        ],
        locked_paths: ['stem', 42],
      },
    })
    expect(data.annotations).toEqual([annotation])
    expect(data.lockedPaths).toEqual(new Set(['stem']))
  })

  it('filters edit indexes without renumbering contour references', () => {
    const contours = [0, 1, 2].map((x) => contour([point(x, 0), point(x, 20)]))
    contours[0]!.id = 'locked'
    const outline = buildEditOutline(contours)
    const filtered = editableOutline(
      outline,
      new Set(['locked']),
      new Set([0, 2]),
      false,
    )
    expect(filtered.contours).toBe(contours)
    expect(filtered.points.map((p) => p.ref.contour)).toEqual([2, 2])
    expect(filtered.segments.every((segment) => segment.contour === 2)).toBe(
      true,
    )
    expect(editableOutline(outline, new Set(), null, true).points).toEqual([])
  })

  it('collects unique selected paths while ignoring other objects', () => {
    expect(
      selectedContours(
        selectionOf([
          { kind: 'node', contour: 2, point: 0 },
          { kind: 'segment', contour: 2, segment: 0 },
          { kind: 'handle', contour: 0, point: 1 },
          { kind: 'component', index: 1 },
        ]),
      ),
    ).toEqual([0, 2])
  })
})
