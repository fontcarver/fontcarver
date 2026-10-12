/**
 * The editor's custom cursors (design spec §2.4).
 *
 * Each is a 24 px SVG drawn as a wide dark stroke with a narrower white stroke
 * on top — the "1 px dark outline + white fill" of the spec — and inlined as a
 * `data:` URI, so nothing is fetched and nothing has to be rasterised at a
 * particular device pixel ratio: an SVG cursor is resolution independent by
 * construction and the browser renders it at the size the platform asks for.
 *
 * Every value ends in a standard cursor name. That is the CSS fallback the
 * platform uses when it refuses the custom image (Wayland without cursor
 * themes, a too-large hotspot, an SVG cursor it does not support), so the
 * pointer always says something even where the drawing cannot be shown.
 */

/** Which custom cursor is wanted. */
export type CursorId =
  | 'pen'
  | 'penAdd'
  | 'penClose'
  | 'penStart'
  | 'knife'
  | 'measure'
  | 'rotate'
  | 'scale'
  | 'insertNode'
  | 'breakHandle'

interface CursorSpec {
  /** SVG body inside a 24 × 24 viewBox. */
  body: string
  /** Hotspot in CSS pixels from the top-left of the image. */
  hotspot: [number, number]
  /** Standard cursor used when the platform rejects the image. */
  fallback: string
}

/** A shape drawn dark-behind-white, which reads on any canvas colour. */
function outlined(path: string, width = 3): string {
  return (
    `<path d="${path}" fill="none" stroke="#14171b" stroke-width="${width}"` +
    ' stroke-linecap="round" stroke-linejoin="round"/>' +
    `<path d="${path}" fill="none" stroke="#ffffff" stroke-width="${width - 1.75}"` +
    ' stroke-linecap="round" stroke-linejoin="round"/>'
  )
}

/** A filled shape with the same dark outline. */
function filled(path: string): string {
  return (
    `<path d="${path}" fill="#ffffff" stroke="#14171b" stroke-width="1.4"` +
    ' stroke-linejoin="round"/>'
  )
}

/** The nib of the pen, its tip on the hotspot. */
const NIB =
  filled('M3 3 L13.5 7.5 L9.5 9.5 L7.5 13.5 Z') + outlined('M7 7 L10 10')

/** A crosshair whose centre is the hotspot. */
const CROSS = outlined('M2 7 H12 M7 2 V12')

const SPECS: Record<CursorId, CursorSpec> = {
  pen: { body: NIB, hotspot: [3, 3], fallback: 'crosshair' },
  penAdd: {
    body: NIB + outlined('M18 13 V22 M13.5 17.5 H22.5'),
    hotspot: [3, 3],
    fallback: 'copy',
  },
  penClose: {
    body: NIB + outlined('M14 14 L22 22 M22 14 L14 22'),
    hotspot: [3, 3],
    fallback: 'crosshair',
  },
  penStart: {
    body:
      NIB +
      '<circle cx="18" cy="18" r="4.2" fill="#ffffff" stroke="#14171b"' +
      ' stroke-width="1.4"/>',
    hotspot: [3, 3],
    fallback: 'crosshair',
  },
  knife: {
    body: filled('M4 2 L15 13 L11.5 16.5 L4 9 Z') + outlined('M15 13 L21 19'),
    hotspot: [4, 2],
    fallback: 'crosshair',
  },
  measure: {
    body:
      CROSS +
      filled('M12 19.5 L19.5 12 L23 15.5 L15.5 23 Z') +
      outlined('M14.5 18 L16 19.5 M17.5 15 L19 16.5', 2.6),
    hotspot: [7, 7],
    fallback: 'crosshair',
  },
  rotate: {
    body:
      CROSS +
      outlined('M22 19 A5.5 5.5 0 1 1 17.5 13.5') +
      filled('M14.5 11 L20 13 L15.5 16.5 Z'),
    hotspot: [7, 7],
    fallback: 'crosshair',
  },
  scale: {
    body:
      CROSS +
      outlined('M14 22 L22 14') +
      filled('M12 23 L12 18 L17 23 Z') +
      filled('M23 12 L23 17 L18 12 Z'),
    hotspot: [7, 7],
    fallback: 'nwse-resize',
  },
  insertNode: {
    body: CROSS + outlined('M18 13 V22 M13.5 17.5 H22.5'),
    hotspot: [7, 7],
    fallback: 'copy',
  },
  breakHandle: {
    body: CROSS + outlined('M13 22 L16.5 18.5') + outlined('M19 16 L22.5 12.5'),
    hotspot: [7, 7],
    fallback: 'crosshair',
  },
}

const cache = new Map<CursorId, string>()

/** The CSS `cursor` value for one custom cursor, with its standard fallback. */
export function toolCursor(id: CursorId): string {
  const cached = cache.get(id)
  if (cached !== undefined) return cached
  const spec = SPECS[id]
  const svg =
    '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"' +
    ` viewBox="0 0 24 24">${spec.body}</svg>`
  const value =
    `url("data:image/svg+xml,${encodeURIComponent(svg)}")` +
    ` ${spec.hotspot[0]} ${spec.hotspot[1]}, ${spec.fallback}`
  cache.set(id, value)
  return value
}

/** The standard cursor a custom one degrades to; used by tests and by the strip. */
export function cursorFallback(id: CursorId): string {
  return SPECS[id].fallback
}
