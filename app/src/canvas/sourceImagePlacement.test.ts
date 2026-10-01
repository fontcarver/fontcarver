import { expect, it } from 'vitest'

import { applyAffine, type Affine } from '../editor/affine'

import { Viewport } from './Viewport'
import type { SourceImageDraw } from './sourceImage'
import {
  beginPlacement,
  placementCursor,
  placementHandleAt,
  placementMarkers,
  updatePlacement,
} from './sourceImagePlacement'

function viewport(): Viewport {
  const view = new Viewport()
  view.setSize(400, 400, 1)
  view.scale = 1
  view.offset = { x: 0, y: 400 }
  return view
}

function image(transform: Affine = [1, 0, 0, 1, 0, 0]): SourceImageDraw {
  return {
    bitmap: {} as HTMLImageElement,
    width: 100,
    height: 50,
    transform,
  }
}

const size = { width: 100, height: 50 }

it('places one handle on every corner, edge and the rotation knob', () => {
  const markers = placementMarkers(viewport(), image())
  expect(markers.map((marker) => marker.handle)).toEqual([
    'scale-sw',
    'scale-se',
    'scale-ne',
    'scale-nw',
    'skew-s',
    'skew-e',
    'skew-n',
    'skew-w',
    'rotate',
  ])
  const corner = markers.find((marker) => marker.handle === 'scale-ne')
  expect(corner?.font).toEqual({ x: 100, y: 50 })
  const rotate = markers.find((marker) => marker.handle === 'rotate')
  // The knob sits a fixed canvas distance beyond the top edge midpoint.
  expect(rotate?.font.x).toBeCloseTo(50)
  expect(rotate?.font.y).toBeCloseTo(76)
})

it('refuses handles for a degenerate or non-finite placement', () => {
  expect(placementMarkers(viewport(), image([0, 0, 0, 0, 0, 0]))).toEqual([])
  expect(placementMarkers(viewport(), image([1, 0, 0, NaN, 0, 0]))).toEqual([])
  expect(placementMarkers(viewport(), { ...image(), width: 0 })).toEqual([])
})

it('hits a handle, then the image body, then nothing', () => {
  const view = viewport()
  const placed = image()
  expect(placementHandleAt(view, placed, { x: 100, y: 400 })).toBe('scale-se')
  expect(placementHandleAt(view, placed, { x: 100, y: 350 })).toBe('scale-ne')
  expect(placementHandleAt(view, placed, { x: 50, y: 375 })).toBe('move')
  expect(placementHandleAt(view, placed, { x: 300, y: 100 })).toBeNull()
  expect(placementCursor('move')).toBe('move')
  expect(placementCursor('scale-ne')).toBe('nwse-resize')
  expect(placementCursor(null)).toBe('default')
})

it('moves the image by the pointer delta', () => {
  const view = viewport()
  const gesture = beginPlacement(view, image(), { x: 50, y: 375 })
  expect(gesture?.handle).toBe('move')
  const next = updatePlacement(gesture!, size, { x: 70, y: 40 }, false)
  expect(next).toEqual([1, 0, 0, 1, 20, 15])
})

it('scales a corner about the opposite corner, uniformly with Shift', () => {
  const view = viewport()
  // Grab the top-right corner at font (100, 50).
  const gesture = beginPlacement(view, image(), { x: 100, y: 350 })
  expect(gesture?.handle).toBe('scale-ne')
  const doubled = updatePlacement(gesture!, size, { x: 200, y: 100 }, false)
  // The south-west corner is the pivot and must not move.
  expect(applyAffine(doubled, { x: 0, y: 0 })).toEqual({ x: 0, y: 0 })
  expect(applyAffine(doubled, { x: 100, y: 50 })).toEqual({ x: 200, y: 100 })
  const uniform = updatePlacement(gesture!, size, { x: 200, y: 60 }, true)
  const corner = applyAffine(uniform, { x: 100, y: 50 })
  expect(corner.x / 100).toBeCloseTo(corner.y / 50)
})

it('skews an edge while the opposite edge stays put', () => {
  const view = viewport()
  const gesture = beginPlacement(view, image(), { x: 50, y: 350 })
  expect(gesture?.handle).toBe('skew-n')
  const next = updatePlacement(gesture!, size, { x: 75, y: 50 }, false)
  // The bottom edge is the pivot; the top edge slides 25 units right.
  expect(applyAffine(next, { x: 0, y: 0 })).toEqual({ x: 0, y: 0 })
  expect(applyAffine(next, { x: 100, y: 0 })).toEqual({ x: 100, y: 0 })
  expect(applyAffine(next, { x: 0, y: 50 }).x).toBeCloseTo(25)
  expect(applyAffine(next, { x: 0, y: 50 }).y).toBeCloseTo(50)
})

it('rotates about the image centre and snaps to 15 degrees with Shift', () => {
  const view = viewport()
  const start = placementMarkers(view, image()).find(
    (marker) => marker.handle === 'rotate',
  )
  const gesture = beginPlacement(view, image(), start!.screen)
  expect(gesture?.handle).toBe('rotate')
  // Drag the knob a quarter turn clockwise, to the right of the centre.
  const next = updatePlacement(gesture!, size, { x: 150, y: 25 }, false)
  const centre = applyAffine(next, { x: 50, y: 25 })
  expect(centre.x).toBeCloseTo(50)
  expect(centre.y).toBeCloseTo(25)
  const top = applyAffine(next, { x: 50, y: 50 })
  expect(top.x).toBeCloseTo(75)
  expect(top.y).toBeCloseTo(25)
  const snapped = updatePlacement(gesture!, size, { x: 60, y: 90 }, true)
  const angle = Math.atan2(snapped[1], snapped[0])
  expect(Math.abs(angle) % (Math.PI / 12)).toBeCloseTo(0)
})

it('keeps the starting placement when a gesture would be degenerate', () => {
  const view = viewport()
  const gesture = beginPlacement(view, image(), { x: 100, y: 350 })
  expect(updatePlacement(gesture!, size, { x: NaN, y: 0 }, false)).toEqual(
    gesture!.start,
  )
  // Dragging the corner onto its own pivot would collapse the image.
  expect(updatePlacement(gesture!, size, { x: 0, y: 0 }, false)).toEqual(
    gesture!.start,
  )
  expect(beginPlacement(view, image(), { x: 390, y: 10 })).toBeNull()
})

it('keeps a rotated placement consistent under its own handles', () => {
  const view = viewport()
  const turned: Affine = [0, 1, -1, 0, 50, 0]
  const markers = placementMarkers(view, image(turned))
  const corner = markers.find((marker) => marker.handle === 'scale-ne')
  expect(corner?.font).toEqual({ x: 0, y: 100 })
  const gesture = beginPlacement(view, image(turned), corner!.screen)
  expect(gesture?.handle).toBe('scale-ne')
  const next = updatePlacement(gesture!, size, { x: -50, y: 200 }, false)
  // The pivot corner holds and the dragged corner follows the pointer.
  expect(applyAffine(next, { x: 0, y: 0 })).toEqual(
    applyAffine(turned, { x: 0, y: 0 }),
  )
  expect(applyAffine(next, { x: 100, y: 50 }).x).toBeCloseTo(-50)
  expect(applyAffine(next, { x: 100, y: 50 }).y).toBeCloseTo(200)
})
