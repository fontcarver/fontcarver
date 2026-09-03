/**
 * Fixtures for the glyph-grid tests: synthetic cells, whole pages, and a jsdom
 * viewport for the scroll container (jsdom reports every element as 0x0, which
 * would leave the virtual grid with nothing to render).
 */

import type { GlyphCell, GlyphListResult } from '../api'

/** Deterministic, unique name for one position in the list. */
export function glyphNameAt(index: number): string {
  return index < 26 ? String.fromCharCode(65 + index) : `g${index}`
}

/** One synthetic cell whose handle equals its position. */
export function cellFor(
  index: number,
  overrides: Partial<GlyphCell> = {},
): GlyphCell {
  return {
    gid: index,
    name: glyphNameAt(index),
    unicodes: [0x41 + index],
    width: 1479,
    bbox: [0, 0, 1400, 1493],
    path: 'M0 0L100 0L100 100Z',
    color_label: null,
    export: true,
    has_components: false,
    is_empty: false,
    generation: 0,
    ...overrides,
  }
}

/** A full page of cells starting at `offset`, clipped to `total`. */
export function glyphPage(
  offset: number,
  limit: number,
  total: number,
  name: (index: number) => string = glyphNameAt,
): GlyphListResult {
  const count = Math.max(0, Math.min(limit, total - offset))
  return {
    total,
    items: Array.from({ length: count }, (_, position) =>
      cellFor(offset + position, { name: name(offset + position) }),
    ),
  }
}

/** Width and height the stubbed scroll container reports. */
export const VIEWPORT = { width: 1200, height: 800 }

/**
 * Makes the element carrying `role="listbox"` report a real size and origin.
 * Returns a disposer that restores jsdom's own behaviour.
 */
export function stubGridViewport(): () => void {
  const isGrid = (element: Element) =>
    element.getAttribute('role') === 'listbox'
  const width = Object.getOwnPropertyDescriptor(
    HTMLElement.prototype,
    'clientWidth',
  )
  const height = Object.getOwnPropertyDescriptor(
    HTMLElement.prototype,
    'clientHeight',
  )
  const rect = HTMLElement.prototype.getBoundingClientRect

  Object.defineProperty(HTMLElement.prototype, 'clientWidth', {
    configurable: true,
    get(this: HTMLElement) {
      return isGrid(this) ? VIEWPORT.width : 0
    },
  })
  Object.defineProperty(HTMLElement.prototype, 'clientHeight', {
    configurable: true,
    get(this: HTMLElement) {
      return isGrid(this) ? VIEWPORT.height : 0
    },
  })
  HTMLElement.prototype.getBoundingClientRect = function boundingRect(
    this: HTMLElement,
  ) {
    if (!isGrid(this)) return rect.call(this)
    return {
      x: 0,
      y: 0,
      left: 0,
      top: 0,
      right: VIEWPORT.width,
      bottom: VIEWPORT.height,
      width: VIEWPORT.width,
      height: VIEWPORT.height,
      toJSON: () => undefined,
    } as DOMRect
  }

  return () => {
    if (width)
      Object.defineProperty(HTMLElement.prototype, 'clientWidth', width)
    if (height)
      Object.defineProperty(HTMLElement.prototype, 'clientHeight', height)
    HTMLElement.prototype.getBoundingClientRect = rect
  }
}
