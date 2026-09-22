import type { Bitmap, BitmapGlyph } from '../../api'
export type PixelTool =
  'pencil' | 'erase' | 'line' | 'rectangle' | 'ellipse' | 'fill' | 'select'
export type PixelPoint = readonly [number, number]
export type PixelSelection = {
  x: number
  y: number
  width: number
  height: number
}
export function blank(width: number, height: number, depth: number): Bitmap {
  if (
    !Number.isInteger(width) ||
    !Number.isInteger(height) ||
    width < 0 ||
    height < 0 ||
    width > 4096 ||
    height > 4096 ||
    ![1, 2, 4, 8, 32].includes(depth)
  )
    throw new Error('Invalid bitmap dimensions')
  const size = Math.ceil((width * depth) / 8) * height
  if (size > 16 * 1024 * 1024) throw new Error('Bitmap exceeds memory limit')
  return {
    width,
    height,
    depth,
    origin_x: 0,
    origin_y: 0,
    rows: new Array<number>(size).fill(0),
  }
}
export function sample(b: Bitmap, x: number, y: number): number {
  if (x < 0 || y < 0 || x >= b.width || y >= b.height) return 0
  const bit = x * b.depth,
    at = y * Math.ceil((b.width * b.depth) / 8) + Math.floor(bit / 8)
  if (b.depth === 32)
    return (
      ((b.rows[at] ?? 0) * 0x1000000 +
        ((b.rows[at + 1] ?? 0) << 16) +
        ((b.rows[at + 2] ?? 0) << 8) +
        (b.rows[at + 3] ?? 0)) >>>
      0
    )
  return ((b.rows[at] ?? 0) >> (8 - b.depth - (bit % 8))) & ((1 << b.depth) - 1)
}
export function put(b: Bitmap, x: number, y: number, value: number): void {
  if (x < 0 || y < 0 || x >= b.width || y >= b.height) return
  const bit = x * b.depth,
    at = y * Math.ceil((b.width * b.depth) / 8) + Math.floor(bit / 8)
  if (b.depth === 32) {
    b.rows.splice(
      at,
      4,
      value >>> 24,
      (value >>> 16) & 255,
      (value >>> 8) & 255,
      value & 255,
    )
    return
  }
  const shift = 8 - b.depth - (bit % 8),
    mask = ((1 << b.depth) - 1) << shift
  b.rows[at] = ((b.rows[at] ?? 0) & ~mask) | ((value << shift) & mask)
}
export function line(
  b: Bitmap,
  start: PixelPoint,
  end: PixelPoint,
  ink: number,
): void {
  let [x, y] = start
  const [ex, ey] = end,
    dx = Math.abs(ex - x),
    dy = -Math.abs(ey - y),
    sx = x < ex ? 1 : -1,
    sy = y < ey ? 1 : -1
  let error = dx + dy
  for (;;) {
    put(b, x, y, ink)
    if (x === ex && y === ey) break
    const twice = 2 * error
    if (twice >= dy) {
      error += dy
      x += sx
    }
    if (twice <= dx) {
      error += dx
      y += sy
    }
  }
}
export function rectangle(a: PixelPoint, b: PixelPoint): PixelSelection {
  return {
    x: Math.min(a[0], b[0]),
    y: Math.min(a[1], b[1]),
    width: Math.abs(a[0] - b[0]) + 1,
    height: Math.abs(a[1] - b[1]) + 1,
  }
}
export function paint(
  source: BitmapGlyph,
  tool: PixelTool,
  start: PixelPoint,
  end: PixelPoint,
  ink: number,
): BitmapGlyph {
  const result = structuredClone(source),
    b = result.bitmap
  if (tool === 'fill') {
    const old = sample(b, ...start)
    if (old === ink) return result
    const stack: PixelPoint[] = [start]
    put(b, ...start, ink)
    while (stack.length) {
      const [x, y] = stack.pop()!
      for (const [nx, ny] of [
        [x - 1, y],
        [x + 1, y],
        [x, y - 1],
        [x, y + 1],
      ] as PixelPoint[]) {
        if (
          nx < 0 ||
          ny < 0 ||
          nx >= b.width ||
          ny >= b.height ||
          sample(b, nx, ny) !== old
        )
          continue
        put(b, nx, ny, ink)
        stack.push([nx, ny])
      }
    }
  } else if (tool === 'ellipse') {
    const r = rectangle(start, end),
      rx = r.width / 2,
      ry = r.height / 2
    const inside = (x: number, y: number) =>
      ((x - r.x + 0.5 - rx) / rx) ** 2 + ((y - r.y + 0.5 - ry) / ry) ** 2 <= 1
    for (let y = r.y; y < r.y + r.height; y++)
      for (let x = r.x; x < r.x + r.width; x++)
        if (
          inside(x, y) &&
          [
            [x - 1, y],
            [x + 1, y],
            [x, y - 1],
            [x, y + 1],
          ].some(([a, b]) => !inside(a, b))
        )
          put(b, x, y, ink)
  } else if (tool === 'rectangle') {
    line(b, start, [end[0], start[1]], ink)
    line(b, [end[0], start[1]], end, ink)
    line(b, end, [start[0], end[1]], ink)
    line(b, [start[0], end[1]], start, ink)
  } else if (tool !== 'select') line(b, start, end, tool === 'erase' ? 0 : ink)
  return result
}
export function transform(
  source: BitmapGlyph,
  kind: 'left' | 'right' | 'up' | 'down' | 'flipX' | 'flipY' | 'rotate',
): BitmapGlyph {
  const b = source.bitmap,
    rotate = kind === 'rotate'
  const out = {
    ...blank(rotate ? b.height : b.width, rotate ? b.width : b.height, b.depth),
    origin_x: b.origin_x,
    origin_y: b.origin_y,
  }
  for (let y = 0; y < b.height; y++)
    for (let x = 0; x < b.width; x++) {
      const nx =
        kind === 'flipX'
          ? b.width - 1 - x
          : rotate
            ? b.height - 1 - y
            : x + (kind === 'left' ? -1 : kind === 'right' ? 1 : 0)
      const ny =
        kind === 'flipY'
          ? b.height - 1 - y
          : rotate
            ? x
            : y + (kind === 'up' ? -1 : kind === 'down' ? 1 : 0)
      put(out, nx, ny, sample(b, x, y))
    }
  return { ...source, bitmap: out, refs: [] }
}
export function resize(
  source: BitmapGlyph,
  width: number,
  height: number,
): BitmapGlyph {
  const out = {
    ...blank(width, height, source.bitmap.depth),
    origin_x: source.bitmap.origin_x,
    origin_y: source.bitmap.origin_y,
  }
  // Preserve baseline coordinates as the canvas grows upwards.
  for (let y = 0; y < height; y++)
    for (let x = 0; x < width; x++)
      put(
        out,
        x,
        y,
        sample(source.bitmap, x, y + source.bitmap.height - height),
      )
  return { ...source, bitmap: out }
}
export function crop(b: Bitmap, region: PixelSelection): Bitmap {
  const result = blank(region.width, region.height, b.depth)
  for (let y = 0; y < region.height; y++)
    for (let x = 0; x < region.width; x++)
      put(result, x, y, sample(b, x + region.x, y + region.y))
  return result
}

/** Transform a selected patch while retaining pixels outside its original bounds. */
export function transformSelection(
  source: BitmapGlyph,
  region: PixelSelection | undefined,
  kind: Parameters<typeof transform>[1],
): BitmapGlyph {
  if (!region) return transform(source, kind)
  const out = structuredClone(source),
    patch = crop(source.bitmap, region)
  for (let y = 0; y < region.height; y++)
    for (let x = 0; x < region.width; x++)
      put(out.bitmap, x + region.x, y + region.y, 0)
  const moving = ['left', 'right', 'up', 'down'].includes(kind)
  const image = moving
    ? patch
    : transform({ ...source, bitmap: patch }, kind).bitmap
  const dx = kind === 'left' ? -1 : kind === 'right' ? 1 : 0,
    dy = kind === 'up' ? -1 : kind === 'down' ? 1 : 0
  for (let y = 0; y < image.height; y++)
    for (let x = 0; x < image.width; x++)
      put(out.bitmap, x + region.x + dx, y + region.y + dy, sample(image, x, y))
  return out
}
