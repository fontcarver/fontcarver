/**
 * The gestures that move the view rather than the outline: panning, the zoom
 * tool's click and marquee, and the selection rubber band.
 *
 * All three are bands or drags with no kernel side at all, and all three have
 * to survive a pointer that never comes back up — a lost capture, a cancelled
 * pointer — without leaking into the next gesture. Keeping them together means
 * `reset()` is one call and the controller's pointer handlers stay about what
 * the *outline* does.
 */

import type { Point2, Viewport } from '../canvas/Viewport'

import { marqueeRect, marqueeSize, type MarqueeRect } from './gestures/marquee'
import { ZOOM_MARQUEE_MIN, zoomAt, zoomToRect } from './tools/zoom'

/** A rubber band in progress, in canvas space. */
export interface Band {
  origin: Point2
  current: Point2
}

/** How a selection marquee was started. */
export interface MarqueeOptions {
  /** Shift or Ctrl: add what the band encloses to the selection. */
  additive: boolean
  /** Alt: leave handles out. Sampled at pointer-down, like `additive`. */
  ignoreHandles: boolean
}

/** A finished selection marquee. */
export interface MarqueeResult extends MarqueeOptions {
  rect: MarqueeRect
}

export class ViewGestures {
  private panning: Point2 | null = null
  private zoomBox: Band | null = null
  private marquee: (Band & MarqueeOptions) | null = null

  /** Whether any view gesture is in progress. */
  get active(): boolean {
    return (
      this.panning !== null || this.zoomBox !== null || this.marquee !== null
    )
  }

  /** Whether the pointer is currently panning. */
  get panningNow(): boolean {
    return this.panning !== null
  }

  /** Whether a selection marquee is being dragged. */
  get marqueeing(): boolean {
    return this.marquee !== null
  }

  /** The band to draw, from either the marquee or the zoom tool. */
  get band(): MarqueeRect | null {
    const band = this.marquee ?? this.zoomBox
    return band ? marqueeRect(band.origin, band.current) : null
  }

  /** Whether the band on screen is a selection marquee. */
  get bandIsMarquee(): boolean {
    return this.marquee !== null
  }

  /** Abandons anything in progress. */
  reset(): void {
    this.panning = null
    this.zoomBox = null
    this.marquee = null
  }

  /** Starts a pan from a canvas point. */
  beginPan(at: Point2): void {
    this.panning = at
  }

  /** Starts the zoom tool's click-or-marquee gesture. */
  beginZoom(at: Point2): void {
    this.zoomBox = { origin: at, current: at }
  }

  /** Starts a selection marquee. */
  beginMarquee(at: Point2, options: MarqueeOptions): void {
    this.marquee = { origin: at, current: at, ...options }
  }

  /**
   * Advances whatever is in progress. Returns `true` when the move belonged to
   * a view gesture and the outline should be left alone.
   */
  move(viewport: Viewport, at: Point2): boolean {
    if (this.panning) {
      viewport.pan(at.x - this.panning.x, at.y - this.panning.y)
      this.panning = at
      return true
    }
    if (this.zoomBox) {
      this.zoomBox = { ...this.zoomBox, current: at }
      return true
    }
    if (this.marquee) {
      this.marquee = { ...this.marquee, current: at }
      return true
    }
    return false
  }

  /** Ends a pan, if one was running. */
  endPan(): boolean {
    if (!this.panning) return false
    this.panning = null
    return true
  }

  /**
   * Ends the zoom gesture: a short drag is a click, which zooms in — or out
   * with Alt — and a longer one frames what it enclosed.
   */
  endZoom(viewport: Viewport, alt: boolean): boolean {
    const box = this.zoomBox
    if (!box) return false
    this.zoomBox = null
    const rect = marqueeRect(box.origin, box.current)
    const size = marqueeSize(rect)
    if (size.width < ZOOM_MARQUEE_MIN && size.height < ZOOM_MARQUEE_MIN)
      zoomAt(viewport, box.origin, alt)
    else zoomToRect(viewport, rect)
    return true
  }

  /** Ends a selection marquee and reports what it enclosed. */
  endMarquee(): MarqueeResult | null {
    const band = this.marquee
    if (!band) return null
    this.marquee = null
    return {
      rect: marqueeRect(band.origin, band.current),
      additive: band.additive,
      ignoreHandles: band.ignoreHandles,
    }
  }
}
