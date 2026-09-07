/**
 * Creating a guide by ⌥-dragging out of a ruler (design spec §7.6).
 *
 * A drag out of the top ruler lays a horizontal guide, one out of the left
 * ruler a vertical one, and ⇧ puts it in the master's global list instead of
 * this layer's. Nothing is committed until the pointer is released, so the
 * gesture is a live preview plus one `AddGuide` — one undo entry, like every
 * other way of adding a guide.
 */

import type { Command, DocId } from '../../api'
import type { Point2, Viewport } from '../../canvas/Viewport'
import { applyCommand } from '../../editor/editSession'
import type { GuideScope } from '../../editor/selection'

import { RULER_SIZE } from './drawRulers'

/** Which ruler the drag came out of. */
export type RulerAxis = 'horizontal' | 'vertical'

/** The ruler a canvas point is inside, or `null` for the canvas proper. */
export function rulerAt(screen: Point2): RulerAxis | null {
  if (screen.x < RULER_SIZE && screen.y < RULER_SIZE) return null
  if (screen.y < RULER_SIZE) return 'horizontal'
  if (screen.x < RULER_SIZE) return 'vertical'
  return null
}

/** What a ruler drag needs to know. */
export interface RulerGuideOptions {
  doc: DocId
  glyph: string
  layer: string
  master: string | null
  scope: GuideScope
  axis: RulerAxis
  /** Undo label, already translated. */
  label: string
}

/** One live "drag a guide off the ruler" gesture. */
export class RulerGuideDrag {
  readonly axis: RulerAxis
  readonly scope: GuideScope
  private readonly options: RulerGuideOptions
  private at: Point2

  constructor(options: RulerGuideOptions, origin: Point2) {
    this.options = options
    this.axis = options.axis
    this.scope = options.scope
    this.at = origin
  }

  /** The font-space x the preview line runs along, or `null` for a horizontal. */
  get previewX(): number | null {
    return this.axis === 'vertical' ? this.at.x : null
  }

  /** The font-space y the preview line runs along, or `null` for a vertical. */
  get previewY(): number | null {
    return this.axis === 'horizontal' ? this.at.y : null
  }

  /** The pointer moved. */
  move(font: Point2): void {
    this.at = font
  }

  /** The command the gesture would commit. */
  command(): Command {
    const { glyph, layer, master, scope } = this.options
    const guide = {
      x: this.axis === 'vertical' ? Math.round(this.at.x) : 0,
      y: this.axis === 'horizontal' ? Math.round(this.at.y) : 0,
      angle: this.axis === 'horizontal' ? 0 : 90,
      name: null,
      locked: false,
      color: null,
    }
    return scope === 'layer'
      ? { AddGuide: { glyph, layer, master: null, guide } }
      : { AddGuide: { glyph: null, layer: null, master, guide } }
  }

  /** Commits the guide as its own undo entry. */
  finish(): Promise<void> {
    return applyCommand(this.options.doc, this.command(), this.options.label)
  }
}

/**
 * Whether a pointer-down should start a ruler drag: ⌥ held, inside a ruler,
 * and the rulers actually on screen.
 */
export function shouldStartRulerGuide(
  screen: Point2,
  modifiers: { alt: boolean },
  rulersVisible: boolean,
): RulerAxis | null {
  if (!modifiers.alt || !rulersVisible) return null
  return rulerAt(screen)
}

/** Font-space position of a canvas point, for the caller's convenience. */
export function fontPoint(viewport: Viewport, screen: Point2): Point2 {
  return viewport.toFont(screen)
}
