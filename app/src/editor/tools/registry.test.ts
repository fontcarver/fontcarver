import { describe, expect, it } from 'vitest'

import { createTool } from '../toolHost'

import { HOLD_BORROW_MS, TOOLS, toolById, toolForKey } from './registry'

describe('tool registry', () => {
  it('lists every tool of design spec §5 in strip order', () => {
    expect(TOOLS.map((tool) => tool.id)).toEqual([
      'select',
      'lasso',
      'pen',
      'pencil',
      'knife',
      'eraser',
      'rectangle',
      'ellipse',
      'measure',
      'rotate',
      'scale',
      'annotation',
      'zoom',
      'hand',
      'text',
    ])
  })

  it('builds a state machine for every tool but Text', () => {
    expect(
      TOOLS.filter((tool) => createTool(tool.id) === null).map(
        (tool) => tool.id,
      ),
    ).toEqual(['select', 'zoom', 'hand', 'text'])
    // Select, Zoom and Hand are the controller's own gestures; Text alone has
    // no behaviour at all yet.
    expect(createTool('text')).toBeNull()
    expect(createTool('pen')).not.toBeNull()
  })

  it('maps the keyboard, with Shift+F reaching the ellipse', () => {
    expect(toolForKey('v', false)).toBe('select')
    expect(toolForKey('F', false)).toBe('rectangle')
    expect(toolForKey('F', true)).toBe('ellipse')
    expect(toolForKey('Z', true)).toBe('zoom')
    expect(toolForKey('q', false)).toBe('lasso')
    expect(toolForKey('e', true)).toBe('eraser')
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
