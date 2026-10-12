import { describe, expect, it } from 'vitest'

import { drawSourceImage, type SourceImageDraw } from './sourceImage'
import { Viewport } from './Viewport'

describe('drawSourceImage', () => {
  it('maps the PNG top edge through image placement and the y-up viewport', () => {
    const calls: unknown[][] = []
    const alpha: number[] = []
    const ctx = {
      globalAlpha: 0.6,
      save() {
        calls.push(['save'])
        alpha.push(this.globalAlpha)
      },
      translate(...args: number[]) {
        calls.push(['translate', ...args])
      },
      scale(...args: number[]) {
        calls.push(['scale', ...args])
      },
      transform(...args: number[]) {
        calls.push(['transform', ...args])
      },
      drawImage(...args: unknown[]) {
        calls.push(['drawImage', ...args, this.globalAlpha])
      },
      restore() {
        calls.push(['restore'])
        this.globalAlpha = alpha.pop() as number
      },
    }
    const viewport = new Viewport()
    viewport.offset = { x: 30, y: 500 }
    viewport.scale = 2
    const image: SourceImageDraw = {
      bitmap: document.createElement('img'),
      width: 120,
      height: 80,
      transform: [0, 1, -1, 0, 15, 25],
    }

    drawSourceImage(ctx as unknown as CanvasRenderingContext2D, viewport, image)

    expect(calls).toEqual([
      ['save'],
      ['translate', 30, 500],
      ['scale', 2, -2],
      ['transform', 0, 1, -1, 0, 15, 25],
      ['translate', 0, 80],
      ['scale', 1, -1],
      ['drawImage', image.bitmap, 0, 0, 120, 80, 0.3],
      ['restore'],
    ])
    expect(ctx.globalAlpha).toBe(0.6)
  })
})
