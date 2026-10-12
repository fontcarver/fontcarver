import { describe, expect, it } from 'vitest'

import { Viewport } from '../../canvas/Viewport'
import { makeContour, makePoint } from '../../test/fontFixtures'
import { createRecordingContext } from '../../test/recordingContext'

import { drawOutline } from './drawOutline'
import { FALLBACK_GLYPH_THEME } from './glyphTheme'

describe('super-smooth node marker', () => {
  it.each([0.25, 1, 4])(
    'keeps the G2 ring separate from the node at zoom %s',
    (scale) => {
      const viewport = new Viewport()
      viewport.scale = scale
      const recording = createRecordingContext()
      drawOutline(
        recording.ctx,
        viewport,
        {
          contours: [
            makeContour(
              [
                makePoint(0, 0, 'Move'),
                { ...makePoint(10, 20, 'Curve', true), super_smooth: true },
                makePoint(30, 40, 'Curve', true),
              ],
              false,
            ),
          ],
          filled: false,
          showPoints: true,
          nodeSize: 8,
          selected: new Set(['node:0:1']),
        },
        FALLBACK_GLYPH_THEME,
      )

      const marked = recording
        .calls('arc')
        .filter(
          (call) => call.args[0] === 10 * scale && call.args[1] === -20 * scale,
        )
      expect(marked.map((call) => call.args[2])).toEqual([4, 3, 6])
      const ordinary = recording
        .calls('arc')
        .filter(
          (call) => call.args[0] === 30 * scale && call.args[1] === -40 * scale,
        )
      expect(ordinary.map((call) => call.args[2])).toEqual([4])
    },
  )
})
