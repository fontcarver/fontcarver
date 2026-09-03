/**
 * The pan and zoom transform shared by every canvas view.
 *
 * Font space is y-up with the baseline at y = 0 and the origin at the glyph's
 * left sidebearing point; canvas space is y-down with the origin at the top
 * left of the element. The class is deliberately plain: it holds no canvas, no
 * DOM node and no React state, so it can be unit tested on its own and reused
 * by the glyph editor, the spacing view and the preview strip later on.
 *
 * `scale` is CSS pixels per font unit and `offset` is where the font-space
 * origin lands, in CSS pixels. The device pixel ratio is carried alongside so
 * a renderer can set its own transform once per frame.
 */

/** A point in either space; the caller knows which. */
export interface Point2 {
  x: number
  y: number
}

/** An axis-aligned box in font units, y-up. */
export interface FontBox {
  x0: number
  y0: number
  x1: number
  y1: number
}

/** Smallest and largest CSS pixels per font unit the viewport allows. */
export const MIN_SCALE = 0.0005
export const MAX_SCALE = 40

function clampScale(scale: number): number {
  if (!Number.isFinite(scale) || scale <= 0) return MIN_SCALE
  return Math.min(MAX_SCALE, Math.max(MIN_SCALE, scale))
}

/** A pan/zoom transform between font space (y-up) and canvas space (y-down). */
export class Viewport {
  /** CSS pixels per font unit. */
  scale = 1
  /** Canvas position of the font-space origin, in CSS pixels. */
  offset: Point2 = { x: 0, y: 0 }
  /** Device pixels per CSS pixel. */
  dpr = 1
  /** Canvas width in CSS pixels. */
  width = 0
  /** Canvas height in CSS pixels. */
  height = 0

  /** Records the canvas size and device pixel ratio. */
  setSize(width: number, height: number, dpr = this.dpr): void {
    this.width = Math.max(0, width)
    this.height = Math.max(0, height)
    this.dpr = dpr > 0 ? dpr : 1
  }

  /** Maps a font-space point to canvas space. */
  toScreen(point: Point2): Point2 {
    return {
      x: this.offset.x + point.x * this.scale,
      y: this.offset.y - point.y * this.scale,
    }
  }

  /** Maps a canvas-space point back to font space. */
  toFont(point: Point2): Point2 {
    return {
      x: (point.x - this.offset.x) / this.scale,
      y: (this.offset.y - point.y) / this.scale,
    }
  }

  /** Moves the view by a canvas-space delta. */
  pan(dx: number, dy: number): void {
    this.offset = { x: this.offset.x + dx, y: this.offset.y + dy }
  }

  /**
   * Scales by `factor` while keeping the font-space point currently under
   * `screen` in place, which is what a wheel or pinch gesture must do.
   */
  zoomAt(screen: Point2, factor: number): void {
    const anchor = this.toFont(screen)
    this.setScaleAt(this.scale * factor, screen, anchor)
  }

  /** Scales to an absolute value, anchored on the centre of the canvas. */
  zoomTo(scale: number): void {
    const screen = { x: this.width / 2, y: this.height / 2 }
    this.setScaleAt(scale, screen, this.toFont(screen))
  }

  /**
   * Frames `box` inside the canvas with `padding` CSS pixels on every side.
   * A degenerate box (an empty glyph, a single point) still yields a usable
   * view because both extents are floored at one font unit.
   */
  fit(box: FontBox, padding = 0): void {
    const boxWidth = Math.max(1e-6, Math.abs(box.x1 - box.x0))
    const boxHeight = Math.max(1e-6, Math.abs(box.y1 - box.y0))
    const available = {
      x: Math.max(1, this.width - padding * 2),
      y: Math.max(1, this.height - padding * 2),
    }
    this.scale = clampScale(
      Math.min(available.x / boxWidth, available.y / boxHeight),
    )
    const centre = { x: (box.x0 + box.x1) / 2, y: (box.y0 + box.y1) / 2 }
    this.offset = {
      x: this.width / 2 - centre.x * this.scale,
      y: this.height / 2 + centre.y * this.scale,
    }
  }

  /** Copies the transform of another viewport, keeping this one's size. */
  copyTransformFrom(other: Viewport): void {
    this.scale = other.scale
    this.offset = { ...other.offset }
  }

  private setScaleAt(scale: number, screen: Point2, anchor: Point2): void {
    const next = clampScale(scale)
    this.scale = next
    this.offset = {
      x: screen.x - anchor.x * next,
      y: screen.y + anchor.y * next,
    }
  }
}
