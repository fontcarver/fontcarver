import { describe, expect, it, vi } from 'vitest'

import { Viewport } from '../../canvas/Viewport'
import type { TextCanvasRun } from '../text/textCanvas'

import {
  BEAM_GRAB_RADIUS,
  beamHeightAt,
  BeamPointer,
  beamScreenY,
  hitCanvasBeam,
} from './beamPointer'

/** One CSS pixel per font unit, with the origin at canvas y 300. */
function viewportAt(): Viewport {
  const viewport = new Viewport()
  viewport.setSize(400, 400, 1)
  viewport.scale = 1
  viewport.offset = { x: 0, y: 300 }
  return viewport
}

function run(overrides: Partial<TextCanvasRun> = {}): TextCanvasRun {
  return {
    glyphs: [],
    cells: new Map(),
    activeIndex: 0,
    activeOrigin: { x: 0, y: 0 },
    mode: 'edit',
    unitsPerEm: 1000,
    fitBox: { x0: 0, y0: 0, x1: 600, y1: 800 },
    requestedScale: 1,
    beam: { height: 100, margins: new Map() },
    onBeamGrab: vi.fn(),
    onActivate: vi.fn(),
    onExitEdit: vi.fn(),
    onVisibleIndexes: vi.fn(),
    ...overrides,
  }
}

describe('grabbing the spacing beam on the glyph canvas', () => {
  it('puts the beam where the run draws it, in the run’s own origin', () => {
    const viewport = viewportAt()
    expect(beamScreenY(run(), viewport)).toBe(200)
    // The beam height is measured from the run's baseline, not the canvas.
    expect(beamScreenY(run({ activeOrigin: { x: 0, y: 40 } }), viewport)).toBe(
      240,
    )
  })

  it('has no beam to grab without one, or without a run that wants it', () => {
    const viewport = viewportAt()
    expect(beamScreenY(run({ beam: null }), viewport)).toBeNull()
    expect(
      beamScreenY(
        run({ beam: { height: Number.NaN, margins: new Map() } }),
        viewport,
      ),
    ).toBeNull()
    expect(
      hitCanvasBeam(run({ beam: null }), viewport, { x: 10, y: 200 }),
    ).toBe(false)
    expect(
      hitCanvasBeam(run({ onBeamGrab: undefined }), viewport, {
        x: 10,
        y: 200,
      }),
    ).toBe(false)
  })

  it('grabs within the radius and lets the glyph through outside it', () => {
    const viewport = viewportAt()
    const beam = run()
    expect(hitCanvasBeam(beam, viewport, { x: 10, y: 200 })).toBe(true)
    expect(
      hitCanvasBeam(beam, viewport, { x: 10, y: 200 + BEAM_GRAB_RADIUS }),
    ).toBe(true)
    expect(
      hitCanvasBeam(beam, viewport, { x: 10, y: 200 + BEAM_GRAB_RADIUS + 1 }),
    ).toBe(false)
  })

  it('reads a height back out of a pointer position', () => {
    const viewport = viewportAt()
    expect(beamHeightAt(run(), viewport, { x: 0, y: 200 })).toBe(100)
    expect(beamHeightAt(run(), viewport, { x: 0, y: 250 })).toBe(50)
    expect(
      beamHeightAt(run({ activeOrigin: { x: 0, y: 40 } }), viewport, {
        x: 0,
        y: 200,
      }),
    ).toBe(140)
    // The round trip is exact: grabbing and releasing without moving cannot
    // shift the beam.
    const height = beamHeightAt(run(), viewport, {
      x: 0,
      y: beamScreenY(run(), viewport) ?? 0,
    })
    expect(height).toBe(100)
  })
})

describe('the beam pointer state machine', () => {
  it('takes the press, the moves and the release, and nothing else', () => {
    const viewport = viewportAt()
    const grab = vi.fn()
    const drag = vi.fn()
    const release = vi.fn()
    const target = run({
      onBeamGrab: grab,
      onBeamDrag: drag,
      onBeamRelease: release,
    })
    const pointer = new BeamPointer()

    // A press away from the line is not the beam's.
    expect(pointer.down(target, viewport, { x: 10, y: 260 })).toBe(false)
    expect(pointer.move(target, viewport, { x: 10, y: 260 })).toBe(false)
    expect(pointer.up(target)).toBe(false)
    expect(grab).not.toHaveBeenCalled()

    expect(pointer.down(target, viewport, { x: 10, y: 202 })).toBe(true)
    expect(pointer.active).toBe(true)
    expect(grab).toHaveBeenCalledOnce()
    expect(pointer.move(target, viewport, { x: 10, y: 250 })).toBe(true)
    expect(drag).toHaveBeenLastCalledWith(50)
    expect(pointer.move(target, viewport, { x: 10, y: 150 })).toBe(true)
    expect(drag).toHaveBeenLastCalledWith(150)
    expect(pointer.up(target)).toBe(true)
    expect(release).toHaveBeenCalledOnce()
    expect(pointer.active).toBe(false)

    // A second release is not a second drag.
    expect(pointer.up(target)).toBe(false)
    expect(release).toHaveBeenCalledOnce()
  })

  it('never reports a drag without a run to report it to', () => {
    const viewport = viewportAt()
    const pointer = new BeamPointer()
    expect(pointer.down(null, viewport, { x: 10, y: 200 })).toBe(false)
    expect(pointer.down(run(), viewport, { x: 10, y: 200 })).toBe(true)
    // The run went away mid-drag: the gesture ends rather than being stranded.
    expect(pointer.move(null, viewport, { x: 10, y: 250 })).toBe(false)
    expect(pointer.active).toBe(false)
  })

  it('leaves a press alone when the run offers no beam hooks', () => {
    const viewport = viewportAt()
    const pointer = new BeamPointer()
    expect(
      pointer.down(run({ onBeamGrab: undefined }), viewport, { x: 10, y: 200 }),
    ).toBe(false)
    expect(pointer.active).toBe(false)
  })
})
