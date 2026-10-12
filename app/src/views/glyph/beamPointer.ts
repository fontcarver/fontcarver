/**
 * Dragging the Spacing tab's measurement beam on the glyph canvas.
 *
 * The beam is drawn by `drawTextGaps` at one font-space height, and the run
 * that owns the beam owns what a new height means; the canvas owns where the
 * line is and what a pointer position measures. That split is why this module
 * is pure: it turns a pointer position into "on the beam or not" and into a
 * height, and the canvas calls the run's three hooks with the answers.
 */

import type { Point2, Viewport } from '../../canvas/Viewport'
import type { TextCanvasRun } from '../text/textCanvas'

/**
 * How far from the beam line a press still grabs it, in CSS pixels.
 *
 * The same order as the design spec's guide radius (§4.2): wide enough to hit
 * without aiming, narrow enough that the band across the run does not swallow
 * clicks meant for the glyphs it crosses.
 */
export const BEAM_GRAB_RADIUS = 5

/** Canvas-space y of the beam line, or `null` when no beam is shown. */
export function beamScreenY(
  run: TextCanvasRun,
  viewport: Viewport,
): number | null {
  const beam = run.beam
  if (!beam || !Number.isFinite(beam.height)) return null
  const y = viewport.toScreen({ x: 0, y: beam.height - run.activeOrigin.y }).y
  return Number.isFinite(y) ? y : null
}

/** Whether a press at `screen` grabs the beam rather than what is under it. */
export function hitCanvasBeam(
  run: TextCanvasRun,
  viewport: Viewport,
  screen: Point2,
): boolean {
  if (!run.onBeamGrab) return false
  const y = beamScreenY(run, viewport)
  return y !== null && Math.abs(screen.y - y) <= BEAM_GRAB_RADIUS
}

/** The beam height a pointer position means, in font units from the baseline. */
export function beamHeightAt(
  run: TextCanvasRun,
  viewport: Viewport,
  screen: Point2,
): number {
  return viewport.toFont(screen).y + run.activeOrigin.y
}

/**
 * One pointer's relationship with the beam, as a three-call state machine.
 *
 * The canvas asks it first on every press, move and release; whenever it
 * answers `true` the beam took the event and the editor controller must not
 * also see it. Keeping the decision here rather than inline in the canvas is
 * what makes "a grab is always followed by a release" testable.
 */
export class BeamPointer {
  private dragging = false

  /** Whether a beam drag is running. */
  get active(): boolean {
    return this.dragging
  }

  /** The press grabbed the beam, so nothing else should act on it. */
  down(run: TextCanvasRun | null, viewport: Viewport, screen: Point2): boolean {
    if (!run || !hitCanvasBeam(run, viewport, screen)) return false
    this.dragging = true
    run.onBeamGrab?.()
    return true
  }

  /** The move belongs to the beam, which is told its new height. */
  move(run: TextCanvasRun | null, viewport: Viewport, screen: Point2): boolean {
    if (!this.dragging) return false
    if (!run) {
      this.dragging = false
      return false
    }
    run.onBeamDrag?.(beamHeightAt(run, viewport, screen))
    return true
  }

  /** The release ends the beam drag; a grab always reaches a release. */
  up(run: TextCanvasRun | null): boolean {
    if (!this.dragging) return false
    this.dragging = false
    run?.onBeamRelease?.()
    return true
  }
}
