import type { ColrGraph, ColrPaint } from '../../api'

const record = (v: unknown): v is Record<string, unknown> =>
  typeof v === 'object' && v !== null && !Array.isArray(v)
const num = (v: unknown): v is number =>
  typeof v === 'number' && Number.isFinite(v)
const tuple = (v: unknown, size: number) =>
  Array.isArray(v) && v.length === size && v.every(num)
const palette = (v: unknown) => v === null || num(v)
const line = (v: unknown) =>
  record(v) &&
  num(v.extend) &&
  Array.isArray(v.stops) &&
  v.stops.every(
    (s) =>
      record(s) && num(s.offset) && palette(s.palette_index) && num(s.alpha),
  )
function transform(v: unknown) {
  if (!record(v)) return false
  switch (v.kind) {
    case 'Affine':
      return tuple(v.matrix, 6)
    case 'Translate':
      return tuple(v.offset, 2)
    case 'Scale':
      return tuple(v.scale, 2) && tuple(v.center, 2)
    case 'Rotate':
      return num(v.angle) && tuple(v.center, 2)
    case 'Skew':
      return tuple(v.angles, 2) && tuple(v.center, 2)
    default:
      return false
  }
}
function paint(v: unknown): v is ColrPaint {
  if (!record(v)) return false
  switch (v.kind) {
    case 'Layers':
      return Array.isArray(v.paints) && v.paints.every(num)
    case 'Solid':
      return palette(v.palette_index) && num(v.alpha)
    case 'Linear':
      return (
        line(v.color_line) &&
        Array.isArray(v.points) &&
        v.points.length === 3 &&
        v.points.every((p) => tuple(p, 2))
      )
    case 'Radial':
      return (
        line(v.color_line) &&
        Array.isArray(v.centers) &&
        v.centers.length === 2 &&
        v.centers.every((p) => tuple(p, 2)) &&
        tuple(v.radii, 2)
      )
    case 'Sweep':
      return line(v.color_line) && tuple(v.center, 2) && tuple(v.angles, 2)
    case 'Glyph':
      return typeof v.glyph === 'string' && num(v.paint)
    case 'ColrGlyph':
      return typeof v.glyph === 'string'
    case 'Transform':
      return num(v.paint) && transform(v.transform)
    case 'Composite':
      return num(v.source) && num(v.backdrop) && num(v.mode)
    default:
      return false
  }
}
/** Structural validation protects draft UI operations; the kernel checks ranges and graph semantics. */
export function parseGraph(text: string): ColrGraph {
  if (text.length > 4 * 1024 * 1024) throw new Error('color.graphInvalid')
  let v: unknown
  try {
    v = JSON.parse(text)
  } catch {
    throw new Error('color.graphInvalid')
  }
  if (
    !record(v) ||
    !record(v.glyphs) ||
    !Object.values(v.glyphs).every(num) ||
    !Array.isArray(v.paints) ||
    v.paints.length > 65535 ||
    !v.paints.every(paint) ||
    (v.clips !== undefined &&
      (!record(v.clips) || !Object.values(v.clips).every((c) => tuple(c, 4))))
  )
    throw new Error('color.graphInvalid')
  return {
    glyphs: v.glyphs as Record<string, number>,
    paints: v.paints,
    clips: (v.clips ?? {}) as ColrGraph['clips'],
  }
}

export function graphPaletteIndices(
  graph: ColrGraph | null | undefined,
): (number | null)[] {
  return (
    graph?.paints.flatMap((p) =>
      p.kind === 'Solid'
        ? [p.palette_index]
        : p.kind === 'Linear' || p.kind === 'Radial' || p.kind === 'Sweep'
          ? p.color_line.stops.map((s) => s.palette_index)
          : [],
    ) ?? []
  )
}
export function shiftGraphPalette(graph: ColrGraph, index: number): ColrGraph {
  const shift = (i: number | null) => (i !== null && i > index ? i - 1 : i)
  return {
    ...graph,
    paints: graph.paints.map((p) =>
      p.kind === 'Solid'
        ? { ...p, palette_index: shift(p.palette_index) }
        : p.kind === 'Linear' || p.kind === 'Radial' || p.kind === 'Sweep'
          ? {
              ...p,
              color_line: {
                ...p.color_line,
                stops: p.color_line.stops.map((s) => ({
                  ...s,
                  palette_index: shift(s.palette_index),
                })),
              },
            }
          : p,
    ),
  }
}

/** A usable two-stop gradient clipped to the named glyph's current outline. */
export function addGradient(
  graph: ColrGraph | null | undefined,
  glyph: string,
  entries: number,
): ColrGraph {
  const previous = graph ?? { glyphs: {}, paints: [], clips: {} }
  return {
    ...previous,
    glyphs: { ...previous.glyphs, [glyph]: previous.paints.length },
    paints: [
      ...previous.paints,
      { kind: 'Glyph', glyph, paint: previous.paints.length + 1 },
      {
        kind: 'Linear',
        points: [
          [0, 0],
          [600, 0],
          [0, 700],
        ],
        color_line: {
          extend: 0,
          stops: [
            { offset: 0, palette_index: 0, alpha: 1 },
            { offset: 1, palette_index: entries > 1 ? 1 : null, alpha: 1 },
          ],
        },
      },
    ],
  }
}
