/**
 * Component drawing.
 *
 * The kernel returns one component-flattened path per layer, which already
 * resolves nested components and their transforms. Rather than re-resolving
 * base glyphs in the browser, the flattened path is drawn first in the muted
 * component colour and the layer's own contours are drawn over it, so what
 * remains visible in that colour is exactly the component contribution. Each
 * component also gets a marker and a label at its own origin.
 */

import type { Component } from '../../api'
import { paintHatch } from '../../canvas/glyphPaint'
import type { Viewport } from '../../canvas/Viewport'
import { IDENTITY, type Affine } from '../../editor/affine'

import type { GlyphTheme } from './glyphTheme'

const MARKER_SIZE = 5
const LABEL_FONT_SIZE = 10
const LABEL_GAP = 2

interface LabelBox {
  left: number
  top: number
  right: number
  bottom: number
}

function intersects(a: LabelBox, b: LabelBox): boolean {
  return (
    a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top
  )
}

function labelPosition(
  x: number,
  preferredY: number,
  width: number,
  placed: readonly LabelBox[],
): { y: number; box: LabelBox } {
  const step = LABEL_FONT_SIZE + LABEL_GAP
  for (let attempt = 0; attempt <= placed.length * 2 + 2; attempt += 1) {
    const distance = Math.ceil(attempt / 2) * step
    const y =
      attempt === 0
        ? preferredY
        : preferredY + (attempt % 2 === 1 ? -distance : distance)
    const box = {
      left: x,
      top: y - LABEL_FONT_SIZE,
      right: x + width,
      bottom: y,
    }
    if (!placed.some((other) => intersects(box, other))) return { y, box }
  }
  const y = preferredY - (placed.length + 1) * step
  return {
    y,
    box: {
      left: x,
      top: y - LABEL_FONT_SIZE,
      right: x + width,
      bottom: y,
    },
  }
}

/** One component as the canvas draws it. */
export interface ComponentDraw {
  /** The stored reference, moved when a gesture is previewing it. */
  component: Component
  /** The component's own flattened path, in font units, or `null`. */
  path: Path2D | null
  /**
   * The transform a live gesture is previewing, in font space, on top of the
   * path — which the kernel already produced under the *stored* transform. A
   * rotate or a scale is a full affine, so a `dx`/`dy` pair would draw a turn
   * as a slide.
   */
  transform: Affine
}

/** What `drawComponents` renders. */
export interface ComponentsData {
  /**
   * Component-flattened outline of the whole layer, used only until the
   * per-component paths have arrived: one path for the layer cannot show a
   * single dragged component moving.
   */
  path: Path2D | null
  /** Keep selectable component markers without painting pre-effect geometry. */
  hidePaths?: boolean
  components: readonly ComponentDraw[]
  /** Flattened bounds per component, in the same order. */
  bounds?: readonly ([number, number, number, number] | null)[]
  /** Index of the component the pointer is over, or `null`. */
  hovered?: number | null
  /** Fill instead of stroking, matching the outline preview mode. */
  filled: boolean
  /** Draw the per-component origin markers and names. */
  showLabels: boolean
}

/**
 * Whether a component is locked in place.
 *
 * An automatically aligned component takes its position from the base glyph's
 * anchors rather than from a stored offset, so dragging it does nothing: it is
 * the model's "locked component", and design spec §4.1 item 6 asks for a hatch
 * over it on hover so the user knows why before they try.
 */
export function componentLocked(component: Component): boolean {
  return component.auto_align
}

/** Draws one path in the component colour, under a preview transform. */
function paintPath(
  ctx: CanvasRenderingContext2D,
  viewport: Viewport,
  path: Path2D,
  transform: Affine,
  filled: boolean,
  theme: GlyphTheme,
): void {
  ctx.save()
  ctx.translate(viewport.offset.x, viewport.offset.y)
  ctx.scale(viewport.scale, -viewport.scale)
  // Canvas takes the same `[a b c d e f]` order the kernel does, and the
  // transform composes in font space, inside the y-up flip above.
  ctx.transform(...transform)
  if (filled) {
    ctx.fillStyle = theme.component
    ctx.fill(path, 'nonzero')
  } else {
    ctx.strokeStyle = theme.component
    ctx.lineWidth = 1 / viewport.scale
    ctx.stroke(path)
  }
  ctx.restore()
}

/** Draws the component contribution of one layer. */
export function drawComponents(
  ctx: CanvasRenderingContext2D,
  viewport: Viewport,
  data: ComponentsData,
  theme: GlyphTheme,
): void {
  if (data.components.length === 0) return

  const own = data.components.filter((entry) => entry.path !== null)
  if (!data.hidePaths && own.length > 0)
    for (const entry of own)
      paintPath(
        ctx,
        viewport,
        entry.path as Path2D,
        entry.transform,
        data.filled,
        theme,
      )
  else if (!data.hidePaths && data.path)
    paintPath(ctx, viewport, data.path, IDENTITY, data.filled, theme)

  // The hatch marks a hovered component the pointer cannot move.
  const hovered = data.hovered == null ? null : data.components[data.hovered]
  const hoveredBox =
    data.hovered == null ? null : (data.bounds?.[data.hovered] ?? null)
  if (hovered && hoveredBox && componentLocked(hovered.component)) {
    const min = viewport.toScreen({ x: hoveredBox[0], y: hoveredBox[1] })
    const max = viewport.toScreen({ x: hoveredBox[2], y: hoveredBox[3] })
    paintHatch(
      ctx,
      {
        x: Math.min(min.x, max.x),
        y: Math.min(min.y, max.y),
        width: Math.abs(max.x - min.x),
        height: Math.abs(max.y - min.y),
      },
      theme.component,
    )
  }

  if (!data.showLabels) return

  ctx.save()
  ctx.strokeStyle = theme.component
  ctx.fillStyle = theme.component
  ctx.lineWidth = 1
  ctx.font = `${LABEL_FONT_SIZE}px ${theme.fontFamily}`
  ctx.textAlign = 'left'
  ctx.textBaseline = 'bottom'
  const labels: LabelBox[] = []
  for (const { component } of data.components) {
    const origin = viewport.toScreen({
      x: component.transform[4],
      y: component.transform[5],
    })
    ctx.beginPath()
    ctx.moveTo(origin.x - MARKER_SIZE, origin.y)
    ctx.lineTo(origin.x + MARKER_SIZE, origin.y)
    ctx.moveTo(origin.x, origin.y - MARKER_SIZE)
    ctx.lineTo(origin.x, origin.y + MARKER_SIZE)
    ctx.stroke()
    const text = component.base
    const x = origin.x + MARKER_SIZE + LABEL_GAP
    const position = labelPosition(
      x,
      origin.y - LABEL_GAP,
      ctx.measureText(text).width,
      labels,
    )
    labels.push(position.box)
    ctx.fillText(text, x, position.y)
  }
  ctx.restore()
}
