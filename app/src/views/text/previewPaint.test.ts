import { describe, expect, it, vi } from 'vitest'

import { createRecordingContext } from '../../test/recordingContext'
import { paintPreview, previewGeometry } from './previewPaint'

const glyphs = [
  { path: 'M0 0L500 0L500 700Z', x: 0, y: 0, xAdvance: 600, yAdvance: 0 },
]

describe('preview paint', () => {
  it('allocates enough height for large horizontal waterfall sizes', () => {
    expect(previewGeometry(glyphs, 256, 1000).cssHeight).toBeGreaterThanOrEqual(
      256,
    )
  })

  it('mirrors device-pixel transforms and scales blur without changing source glyphs', () => {
    const recording = createRecordingContext()
    paintPreview(recording.ctx, glyphs, {
      size: 48,
      unitsPerEm: 1000,
      width: 160,
      height: 70,
      pixelRatio: 2,
      minX: 0,
      ink: 'black',
      flipHorizontal: true,
      flipVertical: true,
      blur: 1.5,
    })
    expect(recording.calls('setTransform')[1]?.args).toEqual([
      -0.096, 0, 0, 0.096, 304, 22,
    ])
    expect(recording.styleAt('fill', 'filter')).toBe('blur(3px)')
    expect(glyphs[0]?.x).toBe(0)
  })

  it('paints color-only glyphs with shaped offsets in the same font transform', () => {
    const recording = createRecordingContext()
    const paint = vi.fn<
      (context: CanvasRenderingContext2D, foreground: string) => void
    >((context) => {
      context.fillStyle = 'red'
      context.fillRect(0, 0, 500, 700)
    })
    paintPreview(
      recording.ctx,
      [{ ...glyphs[0]!, path: '', x: 30, y: 20, color: { paint } }],
      {
        size: 48,
        unitsPerEm: 1000,
        width: 160,
        height: 70,
        pixelRatio: 1,
        minX: 0,
        ink: 'black',
      },
    )
    expect(paint).toHaveBeenCalledOnce()
    expect(paint.mock.calls[0]?.[0]).toBe(recording.ctx)
    expect(paint.mock.calls[0]?.[1]).toBe('black')
    expect(recording.calls('translate')[0]?.args).toEqual([30, 20])
    expect(recording.styleAt('fillRect', 'fillStyle')).toBe('red')
    expect(recording.calls('fill')).toHaveLength(0)
  })
})
