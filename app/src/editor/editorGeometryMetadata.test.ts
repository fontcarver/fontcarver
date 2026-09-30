import { describe, expect, it } from 'vitest'
import { Viewport } from '../canvas/Viewport'
import {
  makeContour,
  makeLayer,
  makeLayerOutline,
  makePoint,
} from '../test/fontFixtures'
import { EditorGeometry } from './editorGeometry'
import type { EditorSource } from './editorTypes'
import { EMPTY_SELECTION } from './selection'

function source(): EditorSource {
  const layer = makeLayer({
    contours: [0, 100].map((x) =>
      makeContour([
        makePoint(x, 0),
        makePoint(x + 40, 0),
        makePoint(x + 40, 40),
        makePoint(x, 40),
      ]),
    ),
  })
  layer.contours[0]!.id = 'protected'
  return {
    doc: 1,
    gid: 1,
    glyph: 'a',
    outline: makeLayerOutline(layer),
    guides: [],
    metrics: {
      upem: 1000,
      ascender: 800,
      descender: -200,
      xHeight: 500,
      capHeight: 700,
      italicAngle: 0,
      width: 600,
      lsb: 0,
      rsb: 460,
      verticalOrigin: null,
      verticalWidth: null,
    },
    master: 'm1',
    curves: 'cubic',
    componentPoints: [],
    editorMetadata: {
      'com.fontcarver.editor': { locked_paths: ['protected'] },
    },
  }
}

describe('editor focus and lock selection', () => {
  it('excludes locked paths from clicking and preserves unlocked stored indexes', () => {
    const geometry = new EditorGeometry()
    geometry.install(source())
    const viewport = new Viewport()
    expect(
      geometry.hitAt(
        viewport,
        EMPTY_SELECTION,
        viewport.toScreen({ x: 0, y: 0 }),
      ),
    ).toBeNull()
    expect(
      geometry.hitAt(
        viewport,
        EMPTY_SELECTION,
        viewport.toScreen({ x: 100, y: 0 }),
      )?.target,
    ).toEqual({ kind: 'node', contour: 1, point: 0 })
    expect(geometry.selectableOutline().points).toHaveLength(4)
  })

  it('clears focus on owner handover and denies every selection on a locked glyph', () => {
    const geometry = new EditorGeometry()
    geometry.install(source())
    geometry.setPathFocus(new Set([0]))
    expect(geometry.canSelect({ kind: 'node', contour: 1, point: 0 })).toBe(
      false,
    )
    expect(geometry.canSelect({ kind: 'anchor', index: 0 })).toBe(false)
    geometry.install({ ...source(), gid: 2, glyph: 'b' })
    expect(geometry.canSelect({ kind: 'node', contour: 1, point: 0 })).toBe(
      true,
    )
    geometry.install({ ...source(), glyphLocked: true })
    expect(geometry.selectableOutline().points).toEqual([])
    expect(geometry.canSelect({ kind: 'component', index: 0 })).toBe(false)
  })
  it('omits locked component hit boxes without renumbering the remaining components', () => {
    const geometry = new EditorGeometry()
    const data = source()
    data.outline.components = [true, false].map((locked) => ({
      component: {
        base: 'a',
        transform: [1, 0, 0, 1, 0, 0],
        anchor: null,
        auto_align: false,
        use_my_metrics: false,
        round_to_grid: false,
        smart: null,
        point_match: null,
        locked,
      },
      path: '',
      bbox: [300, 0, 400, 100],
    }))
    geometry.install(data)
    expect(geometry.canSelect({ kind: 'component', index: 0 })).toBe(false)
    expect(geometry.canSelect({ kind: 'component', index: 1 })).toBe(true)
    const input = geometry.hitInput(new Viewport(), EMPTY_SELECTION)
    expect(input.components.map((component) => component.bbox)).toEqual([
      null,
      [300, 0, 400, 100],
    ])
  })
})
