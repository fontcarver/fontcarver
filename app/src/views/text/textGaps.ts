import type { Point2 } from '../../canvas/Viewport'
import { parseTextInput } from './textInput'
import type { PositionedGlyph } from './textLayout'

/** Why a shaped boundary cannot identify a simple editable pair. */
export type TextGapReason = 'rtl' | 'cluster' | 'named' | 'unresolved'

/** A boundary after `index`, positioned by the kernel's shaped run. */
export interface TextGap {
  index: number
  left: string | null
  right: string | null
  x: number
  /** Present only for top-to-bottom boundaries, measured in font space. */
  y?: number
  reason: TextGapReason | null
}

/** Accept only one resolved glyph per scalar, in a single LTR literal span. */
export function identifyTextGaps(
  text: string,
  glyphs: readonly PositionedGlyph[],
  direction: string | null,
): TextGap[] {
  const vertical = direction === 'ttb'
  const characters = new Map<
    number,
    { end: number; token: number; simple: boolean }
  >()
  const boundaries = new Set(glyphs.map((glyph) => glyph.sourceByte))
  const encoder = new TextEncoder()
  parseTextInput(text).forEach((token, index) => {
    if (token.kind !== 'literal') return
    let byte = 0
    for (const character of token.text) {
      const start = token.sourceByteMap[byte]
      byte += encoder.encode(character).length
      const end = token.sourceByteMap[byte]
      if (start !== undefined && end !== undefined)
        characters.set(start, {
          end,
          token: index,
          simple: !/[\p{M}\p{Cc}\p{Cf}]/u.test(character),
        })
    }
    const end = token.sourceByteMap.at(-1)
    if (end !== undefined) boundaries.add(end)
  })
  const sorted = [...boundaries].sort((a, b) => a - b)
  const nextBoundary = new Map(
    sorted.map((byte, index) => [byte, sorted[index + 1]]),
  )
  const counts = new Map<number, number>()
  for (const glyph of glyphs)
    counts.set(glyph.sourceByte, (counts.get(glyph.sourceByte) ?? 0) + 1)
  const simple = (glyph: PositionedGlyph) => {
    const character = characters.get(glyph.sourceByte)
    return (
      character?.simple &&
      counts.get(glyph.sourceByte) === 1 &&
      nextBoundary.get(glyph.sourceByte) === character.end &&
      (vertical
        ? glyph.yAdvance < 0 && glyph.xAdvance === 0
        : glyph.xAdvance > 0 && glyph.yAdvance === 0 && glyph.y === 0)
    )
  }
  return glyphs.slice(0, -1).map((left, index): TextGap => {
    const right = glyphs[index + 1]!
    const first = characters.get(left.sourceByte)
    const second = characters.get(right.sourceByte)
    const reason =
      direction === 'rtl'
        ? 'rtl'
        : left.sourceKind === 'named' || right.sourceKind === 'named'
          ? 'named'
          : (vertical && right.penY === undefined) ||
              !left.editable ||
              !right.editable ||
              left.documentGid === null ||
              right.documentGid === null ||
              left.name === null ||
              right.name === null
            ? 'unresolved'
            : !simple(left) ||
                !simple(right) ||
                first?.token !== second?.token ||
                first?.end !== right.sourceByte ||
                (vertical ? right.y >= left.y : right.x <= left.x)
              ? 'cluster'
              : null
    return {
      index,
      left: left.name,
      right: right.name,
      x: right.x,
      reason,
      ...(vertical ? { y: right.penY, x: right.penX ?? right.x } : {}),
    }
  })
}

/** A six-screen-pixel strip at the shaped boundary; no spacing is invented. */
export function hitTextGap(
  gaps: readonly TextGap[],
  point: Point2,
  unitsPerEm: number,
  scale: number,
): number | null {
  const vertical = gaps.some((gap) => gap.y !== undefined)
  if (!vertical && (point.y < -unitsPerEm * 0.2 || point.y > unitsPerEm * 0.8))
    return null
  const tolerance = 6 / Math.max(scale, 0.0005)
  let closest: TextGap | undefined
  for (const gap of gaps) {
    const distance = (candidate: TextGap) =>
      candidate.y === undefined
        ? Math.abs(point.x - candidate.x)
        : Math.abs(point.y - candidate.y)
    if (
      gap.reason !== null ||
      distance(gap) > tolerance ||
      (gap.y !== undefined &&
        (point.x < gap.x - unitsPerEm / 2 || point.x > gap.x + unitsPerEm / 2))
    )
      continue
    if (!closest || distance(gap) < distance(closest)) closest = gap
  }
  return closest?.index ?? null
}

/**
 * The kerning the shaped run actually applied after one glyph, in font units.
 *
 * The shaped advance already carries every positioning rule the compiled font
 * contains, so the difference from the glyph's own stored advance is what was
 * added at that boundary — regardless of whether it came from an editable pair,
 * a class rule or an imported layout table. `null` means the glyph's advance is
 * not known yet, which happens while its cell is still loading. Differences
 * below half a unit read as zero so rounding never paints a colour.
 */
export function appliedKerning(
  left: PositionedGlyph | undefined,
  storedAdvance: number | undefined,
  vertical: boolean,
): number | null {
  if (
    !left ||
    storedAdvance === undefined ||
    !Number.isFinite(storedAdvance) ||
    left.documentGid === null
  )
    return null
  const shaped = vertical ? -left.yAdvance : left.xAdvance
  if (!Number.isFinite(shaped)) return null
  const delta = shaped - storedAdvance
  return Math.abs(delta) < 0.5 ? 0 : delta
}
