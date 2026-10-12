import { expect, it, vi } from 'vitest'

import type { Component, ComponentView, SmartAxis } from '../../../api'
import { Viewport } from '../../../canvas/Viewport'
import { selectionOf } from '../../../editor/selection'

import {
  MAX_CANVAS_AXES,
  smartHandleAt,
  smartHandles,
  smartValueFor,
} from './smartComponentCanvas'
import { SmartComponentPointer } from './smartComponentPointer'

vi.mock('../commitField', () => ({
  commitField: vi.fn(
    (
      target: unknown,
      command: unknown,
      label: string,
      onCommitted: () => void,
    ) => {
      commits.push({ target, command, label })
      onCommitted()
    },
  ),
}))

const commits: { target: unknown; command: unknown; label: string }[] = []

function viewport(): Viewport {
  const view = new Viewport()
  view.setSize(600, 600, 1)
  view.scale = 1
  view.offset = { x: 0, y: 600 }
  return view
}

const axes: SmartAxis[] = [
  { name: 'width', min: 0, max: 100, default: 50 },
  { name: 'height', min: 0, max: 10, default: 5 },
]

function component(overrides: Partial<Component> = {}): Component {
  return {
    base: 'radical',
    transform: [1, 0, 0, 1, 0, 0],
    anchor: null,
    auto_align: false,
    locked: false,
    use_my_metrics: false,
    round_to_grid: false,
    point_match: null,
    smart: null,
    ...overrides,
  }
}

function view(overrides: Partial<ComponentView> = {}): ComponentView {
  return {
    smart_axes: axes,
    component: component(),
    path: '',
    bbox: [100, 0, 300, 400],
    ...overrides,
  }
}

const selected = selectionOf([{ kind: 'component', index: 0 }])

it('tracks the first axis along the bottom edge and the second along the left', () => {
  const handles = smartHandles([view()], selected, viewport())
  expect(handles).toHaveLength(2)
  expect(handles[0]).toMatchObject({
    component: 0,
    axis: 0,
    name: 'width',
    from: { x: 100, y: -12 },
    to: { x: 300, y: -12 },
  })
  // The default of 50 on a 0..100 axis is the middle of the track.
  expect(handles[0]?.at).toEqual({ x: 200, y: -12 })
  expect(handles[1]).toMatchObject({
    axis: 1,
    name: 'height',
    from: { x: 88, y: 0 },
    to: { x: 88, y: 400 },
  })
  expect(handles[1]?.at).toEqual({ x: 88, y: 200 })
})

it('draws no handle without a selection, local axes, bounds or a lock', () => {
  const empty = new Map()
  expect(smartHandles([view()], empty, viewport())).toEqual([])
  expect(
    smartHandles([view({ smart_axes: [] })], selected, viewport()),
  ).toEqual([])
  expect(smartHandles([view({ bbox: null })], selected, viewport())).toEqual([])
  expect(
    smartHandles(
      [view({ component: component({ locked: true }) })],
      selected,
      viewport(),
    ),
  ).toEqual([])
  expect(smartHandles([view()], selected, viewport(), true)).toEqual([])
})

it('caps the drawn tracks and leaves the rest to the inspector', () => {
  const many: SmartAxis[] = Array.from({ length: 6 }, (_, index) => ({
    name: `a${index}`,
    min: 0,
    max: 1,
    default: 0,
  }))
  const handles = smartHandles(
    [view({ smart_axes: many })],
    selected,
    viewport(),
  )
  expect(handles).toHaveLength(MAX_CANVAS_AXES)
})

it('maps a pointer position onto the axis range, clamped and stepped', () => {
  const handles = smartHandles([view()], selected, viewport())
  const width = handles[0]!
  expect(smartValueFor(width, { x: 100, y: -12 })).toBe(0)
  expect(smartValueFor(width, { x: 300, y: -12 })).toBe(100)
  expect(smartValueFor(width, { x: 250, y: 400 })).toBe(75)
  expect(smartValueFor(width, { x: -500, y: 0 })).toBe(0)
  expect(smartValueFor(width, { x: 5000, y: 0 })).toBe(100)
  // A vertical track answers only to vertical movement.
  const height = handles[1]!
  expect(smartValueFor(height, { x: 5000, y: 100 })).toBe(2.5)
})

it('finds the handle under the pointer and nothing further away', () => {
  const view3 = viewport()
  const handles = smartHandles([view()], selected, view3)
  // Font (200, -12) is canvas (200, 612).
  expect(smartHandleAt(handles, view3, { x: 200, y: 612 })?.name).toBe('width')
  expect(smartHandleAt(handles, view3, { x: 220, y: 612 })).toBeNull()
})

it('commits one SetComponent per drag and keeps the other axis', () => {
  commits.length = 0
  const pointer = new SmartComponentPointer()
  const views = [view({ component: component({ smart: { height: 7 } }) })]
  const view4 = viewport()
  const target = { doc: 1, glyph: 'A', layer: 'm1' }
  const reload = vi.fn()
  expect(
    pointer.down(views, selected, view4, false, { x: 200, y: 612, button: 0 }),
  ).toBe(true)
  expect(pointer.busy).toBe(true)
  pointer.move(views, selected, view4, false, { x: 300, y: 612, button: 0 })
  expect(pointer.active()?.value).toBe(100)
  expect(pointer.up(views, target, reload)).toBe(true)
  expect(reload).toHaveBeenCalledTimes(1)
  expect(commits).toHaveLength(1)
  expect(commits[0]?.command).toEqual({
    SetComponent: {
      glyph: 'A',
      layer: 'm1',
      index: 0,
      component: { ...views[0]!.component, smart: { width: 100, height: 7 } },
    },
  })
})

it('writes nothing for a press that does not move, or after a cancel', () => {
  commits.length = 0
  const pointer = new SmartComponentPointer()
  const views = [view()]
  const view5 = viewport()
  const target = { doc: 1, glyph: 'A', layer: 'm1' }
  pointer.down(views, selected, view5, false, { x: 200, y: 612, button: 0 })
  expect(pointer.up(views, target, vi.fn())).toBe(true)
  expect(commits).toHaveLength(0)
  pointer.down(views, selected, view5, false, { x: 200, y: 612, button: 0 })
  pointer.move(views, selected, view5, false, { x: 300, y: 612, button: 0 })
  pointer.cancel()
  expect(pointer.up(views, target, vi.fn())).toBe(false)
  expect(commits).toHaveLength(0)
  // A press that is not on a handle is not this gesture's.
  expect(
    pointer.down(views, selected, view5, false, { x: 10, y: 10, button: 0 }),
  ).toBe(false)
  expect(
    pointer.down(views, selected, view5, false, { x: 200, y: 612, button: 2 }),
  ).toBe(false)
})

it('drops a value back to the axis default rather than storing it', () => {
  commits.length = 0
  const pointer = new SmartComponentPointer()
  const views = [view({ component: component({ smart: { width: 100 } }) })]
  const view6 = viewport()
  const handles = smartHandles(views, selected, view6)
  const at = view6.toScreen(handles[0]!.at)
  pointer.down(views, selected, view6, false, { ...at, button: 0 })
  // Back to the axis default of 50, which is stored as no coordinate at all.
  pointer.move(views, selected, view6, false, { x: 200, y: 612, button: 0 })
  pointer.up(views, { doc: 1, glyph: 'A', layer: 'm1' }, vi.fn())
  expect(commits).toHaveLength(1)
  expect(
    (commits[0]?.command as { SetComponent: { component: Component } })
      .SetComponent.component.smart,
  ).toBeNull()
})
