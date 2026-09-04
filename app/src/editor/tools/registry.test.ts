import { describe, expect, it } from 'vitest'

import { HOLD_BORROW_MS, TOOLS, toolById, toolForKey } from './registry'

describe('tool registry', () => {
  it('lists every tool of design spec §5 in strip order', () => {
    expect(TOOLS.map((tool) => tool.id)).toEqual([
      'select',
      'pen',
      'pencil',
      'knife',
      'rectangle',
      'ellipse',
      'measure',
      'rotate',
      'scale',
      'zoom',
      'hand',
      'text',
    ])
  })

  it('marks Select, Zoom and Hand as the tools that work in this phase', () => {
    expect(TOOLS.filter((tool) => tool.live).map((tool) => tool.id)).toEqual([
      'select',
      'zoom',
      'hand',
    ])
  })

  it('maps the keyboard, with Shift+F reaching the ellipse', () => {
    expect(toolForKey('v', false)).toBe('select')
    expect(toolForKey('F', false)).toBe('rectangle')
    expect(toolForKey('F', true)).toBe('ellipse')
    expect(toolForKey('Z', true)).toBe('zoom')
    expect(toolForKey('q', false)).toBeNull()
  })

  it('borrows a tool after 200 ms of holding its key', () => {
    expect(HOLD_BORROW_MS).toBe(200)
  })

  it('gives every tool a cursor and an i18n key', () => {
    for (const tool of TOOLS) {
      expect(tool.cursor.length).toBeGreaterThan(0)
      expect(tool.labelKey.startsWith('tool.')).toBe(true)
      expect(toolById(tool.id)).toBe(tool)
    }
  })
})
