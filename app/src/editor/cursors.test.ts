import { describe, expect, it } from 'vitest'

import { cursorFallback, toolCursor, type CursorId } from './cursors'

const ALL: CursorId[] = [
  'pen',
  'penAdd',
  'penClose',
  'penStart',
  'knife',
  'measure',
  'rotate',
  'scale',
  'insertNode',
  'breakHandle',
]

describe('toolCursor', () => {
  it('inlines a 24 px SVG with a hotspot and a standard fallback', () => {
    const value = toolCursor('pen')
    expect(value.startsWith('url("data:image/svg+xml,')).toBe(true)
    expect(decodeURIComponent(value)).toContain('width="24" height="24"')
    expect(value.endsWith(' 3 3, crosshair')).toBe(true)
  })

  it('gives every cursor a hotspot inside the image and a fallback', () => {
    for (const id of ALL) {
      const value = toolCursor(id)
      const tail = value.slice(value.lastIndexOf(')') + 1)
      const match = /^ (\d+) (\d+), ([a-z-]+)$/.exec(tail)
      expect(match, id).not.toBeNull()
      expect(Number(match?.[1])).toBeLessThan(24)
      expect(Number(match?.[2])).toBeLessThan(24)
      expect(match?.[3]).toBe(cursorFallback(id))
    }
  })

  it('escapes the SVG so the data URI carries no raw markup', () => {
    for (const id of ALL) {
      const value = toolCursor(id)
      const uri = value.slice(0, value.lastIndexOf(')'))
      expect(uri.includes('<'), id).toBe(false)
      expect(uri.includes('#'), id).toBe(false)
    }
  })

  it('builds each value once', () => {
    expect(toolCursor('knife')).toBe(toolCursor('knife'))
  })
})
