import { describe, expect, it } from 'vitest'
import { blank } from '../views/bitmap/pixels'
import { bitmapFrame, bitmapPointer } from './bitmapGeometry'

describe('bitmap canvas coordinates', () => {
  it('includes baseline, advance and negative origins in the visible frame', () => {
    const b = { ...blank(7, 5, 1), origin_x: -2, origin_y: -1 }
    expect(bitmapFrame(b, 9)).toEqual({
      left: -3,
      top: 5,
      width: 13,
      height: 7,
    })
  })
  it('maps pointer positions to source pixels in a padded y-up frame', () => {
    const b = { ...blank(7, 5, 1), origin_x: -2, origin_y: -1 }
    const frame = bitmapFrame(b, 9)
    expect(bitmapPointer(b, frame, 1.5, 1.5)).toEqual([0, 0])
    expect(bitmapPointer(b, frame, 7.5, 5.5)).toEqual([6, 4])
    expect(bitmapPointer(b, frame, 0.5, 0.5)).toEqual([-1, -1])
  })
  it('includes resolved component pixels without changing the source origin', () => {
    const source = blank(2, 2, 1)
    const composite = { ...blank(8, 8, 1), origin_x: -3, origin_y: -4 }
    expect(bitmapFrame(source, 4, composite)).toEqual({
      left: -4,
      top: 5,
      width: 10,
      height: 10,
    })
  })
})
