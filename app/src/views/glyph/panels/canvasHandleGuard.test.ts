import { expect, it } from 'vitest'

import { TOOLS } from '../../../editor/tools/registry'

import { canvasHandlesActive } from './canvasHandleGuard'

it('keeps the canvas handles away from the view gestures', () => {
  expect(canvasHandlesActive('hand', false)).toBe(false)
  expect(canvasHandlesActive('zoom', false)).toBe(false)
  // Space is "pan while held", whatever tool is selected.
  expect(canvasHandlesActive('select', true)).toBe(false)
  expect(canvasHandlesActive('magnet', true)).toBe(false)
})

it('offers the handles to every editing tool', () => {
  const refused = TOOLS.map((tool) => tool.id).filter(
    (id) => !canvasHandlesActive(id, false),
  )
  expect(refused).toEqual(['zoom', 'hand'])
})
