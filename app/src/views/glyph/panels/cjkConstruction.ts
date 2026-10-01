import type { Command, Component } from '../../../api'
import type { LayerTarget } from '../../../editor/pointCommands'

/** One scalar or bracketed glyph name with its font-space placement. */
export type IdsLeaf = { token: string; transform: Component['transform'] }

/** Stable parser categories, translated by the caller. */
export class IdsError extends Error {
  constructor(readonly code: 'invalid' | 'limit' | 'operator') {
    super(code)
    this.name = 'IdsError'
  }
}

type Cell = { x: number; y: number; width: number; height: number }

const full: Cell = { x: 0, y: 0, width: 1, height: 1 }
const maxScalars = 512
const maxLeaves = 64
const maxDepth = 16

/**
 * Enclosures preserve the outer glyph and reserve inset space for the inner
 * glyph. These geometric templates are starting positions for manual editing.
 */
const interiors: Record<string, Cell> = {
  '⿴': { x: 0.25, y: 0.25, width: 0.5, height: 0.5 },
  '⿵': { x: 0.25, y: 0, width: 0.5, height: 0.75 },
  '⿶': { x: 0.25, y: 0.25, width: 0.5, height: 0.75 },
  '⿷': { x: 0.25, y: 0.25, width: 0.75, height: 0.5 },
  '⿸': { x: 0.25, y: 0, width: 0.75, height: 0.75 },
  '⿹': { x: 0, y: 0, width: 0.75, height: 0.75 },
  '⿺': { x: 0.25, y: 0.25, width: 0.75, height: 0.75 },
  '⿻': full,
}

function cells(operator: string): Cell[] {
  if (operator === '⿰' || operator === '⿲') {
    const count = operator === '⿲' ? 3 : 2
    return Array.from({ length: count }, (_, index) => ({
      x: index / count,
      y: 0,
      width: 1 / count,
      height: 1,
    }))
  }
  if (operator === '⿱' || operator === '⿳') {
    const count = operator === '⿳' ? 3 : 2
    return Array.from({ length: count }, (_, index) => ({
      x: 0,
      y: (count - index - 1) / count,
      width: 1,
      height: 1 / count,
    }))
  }
  return [full, interiors[operator]]
}

function nested(parent: Cell, child: Cell): Cell {
  return {
    x: parent.x + child.x * parent.width,
    y: parent.y + child.y * parent.height,
    width: parent.width * child.width,
    height: parent.height * child.height,
  }
}

function unsupported(point: number): boolean {
  return (point >= 0x2ffc && point <= 0x2fff) || point === 0x31ef
}

/**
 * Parse bounded prefix IDS and lay out leaves relative to the complete em cell:
 * x = 0…width, y = descender…descender + upem. Input order is retained, including
 * top before bottom. Brackets remain in tokens to distinguish explicit glyph
 * names from Unicode lookup; whitespace between tokens is ignored.
 */
export function layoutIds(
  text: string,
  width: number,
  upem: number,
  descender: number,
): IdsLeaf[] {
  if (
    !Number.isFinite(width) ||
    width <= 0 ||
    !Number.isFinite(upem) ||
    upem <= 0 ||
    !Number.isFinite(descender) ||
    !Number.isFinite(descender + upem)
  )
    throw new IdsError('invalid')

  // Stop before allocating an unbounded scalar array, including for astral text.
  const scalars: string[] = []
  for (const scalar of text) {
    if (scalars.length >= maxScalars) throw new IdsError('limit')
    const point = scalar.codePointAt(0) ?? 0
    if (point >= 0xd800 && point <= 0xdfff) throw new IdsError('invalid')
    scalars.push(scalar)
  }
  const leaves: IdsLeaf[] = []
  let cursor = 0
  const skipSpace = () => {
    while (cursor < scalars.length && /\s/u.test(scalars[cursor])) cursor += 1
  }
  const read = (cell: Cell, depth: number): void => {
    skipSpace()
    let token = scalars[cursor++]
    if (token === undefined) throw new IdsError('invalid')
    const point = token.codePointAt(0) ?? 0
    if (unsupported(point)) throw new IdsError('operator')
    if (point >= 0x2ff0 && point <= 0x2ffb) {
      if (depth >= maxDepth) throw new IdsError('limit')
      for (const child of cells(token)) read(nested(cell, child), depth + 1)
      return
    }
    if (token === '[') {
      const start = cursor - 1
      while (cursor < scalars.length && scalars[cursor] !== ']') {
        const part = scalars[cursor]
        const partPoint = part.codePointAt(0) ?? 0
        if (part === '[' || partPoint < 0x20 || partPoint === 0x7f) {
          throw new IdsError('invalid')
        }
        cursor += 1
      }
      if (
        cursor === scalars.length ||
        !scalars
          .slice(start + 1, cursor)
          .join('')
          .trim()
      ) {
        throw new IdsError('invalid')
      }
      cursor += 1
      token = scalars.slice(start, cursor).join('')
    } else if (token === ']' || point < 0x20 || point === 0x7f) {
      throw new IdsError('invalid')
    }
    if (leaves.length >= maxLeaves) throw new IdsError('limit')
    leaves.push({
      token,
      transform: [
        cell.width,
        0,
        0,
        cell.height,
        cell.x * width,
        cell.y * upem + (1 - cell.height) * descender,
      ],
    })
  }
  read(full, 0)
  skipSpace()
  if (cursor !== scalars.length) {
    throw new IdsError(
      unsupported(scalars[cursor].codePointAt(0) ?? 0) ? 'operator' : 'invalid',
    )
  }
  return leaves
}

/** Append components in one undoable operation, preserving their metadata. */
export function idsCommand(
  target: LayerTarget,
  components: readonly Component[],
): Command {
  return {
    Batch: components.map((component) => ({
      AddComponent: { glyph: target.glyph, layer: target.layer, component },
    })),
  }
}
