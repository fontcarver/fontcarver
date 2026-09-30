/** An ordered SVG point used only for source-overlay inspection. */
export interface SourcePoint {
  x: number
  y: number
  kind: 'on' | 'control'
}

/** Parsed SVG topology, including off-curve points and contour closure. */
export interface SourcePoints {
  contours: { points: SourcePoint[]; closed: boolean }[]
  pointCount: number
}

/**
 * Read the absolute M/L/Q/C/Z subset emitted by kurbo, with explicit work limits.
 * Unsupported or malformed input keeps its outline but receives no point links.
 */
export function parseSourcePoints(path: string): SourcePoints | null {
  if (path.length > 100_000) return null
  const token = /[A-Za-z]|[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?/y
  const separator = /[\s,]+/y
  const tokens: (string | number)[] = []
  let position = 0
  while (position < path.length) {
    separator.lastIndex = position
    const space = separator.exec(path)
    if (space) position = separator.lastIndex
    if (position === path.length) break
    token.lastIndex = position
    const match = token.exec(path)
    if (!match) return null
    const value = match[0]
    if (/^[A-Za-z]$/.test(value)) tokens.push(value)
    else {
      const number = Number(value)
      if (!Number.isFinite(number)) return null
      tokens.push(number)
    }
    position = token.lastIndex
  }
  const result: SourcePoints = { contours: [], pointCount: 0 }
  let index = 0
  let command = ''
  let contour: SourcePoints['contours'][number] | undefined
  while (index < tokens.length) {
    if (typeof tokens[index] === 'string') command = tokens[index++] as string
    if (command === 'Z') {
      if (!contour || contour.closed) return null
      contour.closed = true
      command = ''
      continue
    }
    const pairs =
      command === 'M' || command === 'L'
        ? 1
        : command === 'Q'
          ? 2
          : command === 'C'
            ? 3
            : 0
    if (!pairs || index + pairs * 2 > tokens.length) return null
    if (command === 'M') {
      contour = { points: [], closed: false }
      result.contours.push(contour)
    }
    if (!contour || contour.closed) return null
    for (let pair = 0; pair < pairs; pair++) {
      const x = tokens[index++]
      const y = tokens[index++]
      if (typeof x !== 'number' || typeof y !== 'number') return null
      if (++result.pointCount > 2048) return null
      contour.points.push({ x, y, kind: pair === pairs - 1 ? 'on' : 'control' })
    }
    if (command === 'M') command = 'L'
  }
  return result
}

/** Point structure is visual correspondence, not a replacement for API diagnostics. */
export function samePointStructure(
  left: SourcePoints,
  right: SourcePoints,
): boolean {
  return (
    left.contours.length === right.contours.length &&
    left.contours.every((contour, i) => {
      const other = right.contours[i]!
      return (
        contour.closed === other.closed &&
        contour.points.length === other.points.length &&
        contour.points.every((point, j) => point.kind === other.points[j]!.kind)
      )
    })
  )
}
