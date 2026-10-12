import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { setTransport, type EditApplyParams, type Contour } from '../api'
import { makeContour, makePoint } from '../test/fontFixtures'
import {
  createMockTransport,
  noChange,
  type MockTransport,
} from '../test/mockTransport'

import { harmonizeNodes, maintainSuperSmoothContours } from './harmonize'
import { NudgeRunner, type NudgePress } from './nudgeRun'
import { translatePoints } from './outlineGeometry'

function retained(): Contour {
  const contour = makeContour(
    [
      makePoint(-100, 100, 'Move'),
      makePoint(-80, 40, 'OffCurve'),
      makePoint(-30, 0, 'OffCurve'),
      { ...makePoint(0, 0, 'Curve'), super_smooth: true },
      makePoint(50, 0, 'OffCurve'),
      makePoint(70, 10, 'OffCurve'),
      makePoint(100, 100, 'Curve'),
    ],
    false,
  )
  return harmonizeNodes(contour, [3])!
}

function commandContours(mock: MockTransport): Contour[][] {
  return mock.calls
    .filter((entry) => entry.method === 'doc.edit_apply')
    .map((entry) => {
      const { command } = entry.params as EditApplyParams
      expect(command).toHaveProperty('ReplaceLayerOutline')
      return 'ReplaceLayerOutline' in command
        ? command.ReplaceLayerOutline.contours
        : []
    })
}

function setup() {
  const mock = createMockTransport({
    'doc.edit_begin': () => 3,
    'doc.edit_apply': () => noChange([1]),
    'doc.edit_end': () => null,
  })
  setTransport(mock)
  const onError = vi.fn()
  const runner = new NudgeRunner({ onEnd: vi.fn(), onError })
  const input: NudgePress = {
    target: { doc: 1, glyph: 'a', layer: 'layer-1' },
    contours: [retained()],
    anchors: [],
    components: [],
    refs: [{ contour: 0, point: 5 }],
    handles: 'PointsOnly',
    delta: { x: 0, y: 1 },
    label: (count) => `Nudge ${count} points`,
  }
  return { mock, runner, input, onError }
}

beforeEach(() => vi.useFakeTimers())

afterEach(() => {
  vi.useRealTimers()
  setTransport(undefined)
})

describe('G2 nudge previews', () => {
  it('commits cumulative projected geometry across frames in one edit', async () => {
    const { mock, runner, input, onError } = setup()
    const first = runner.press(input)
    await vi.advanceTimersByTimeAsync(30)
    const second = runner.press({ ...input, delta: { x: 3, y: 4 } })
    await vi.advanceTimersByTimeAsync(30)
    await runner.end()

    expect(commandContours(mock)).toEqual([first.contours, second.contours])
    expect(second.contours).toEqual(
      maintainSuperSmoothContours(
        translatePoints(input.contours, input.refs, 3, 5),
      ),
    )
    expect(
      mock.calls.filter((entry) => entry.method === 'doc.edit_begin'),
    ).toHaveLength(1)
    expect(
      mock.calls.filter((entry) => entry.method === 'doc.edit_end'),
    ).toHaveLength(1)
    expect(onError).not.toHaveBeenCalled()
  })

  it('keeps the last valid preview when a later press creates an inflection', async () => {
    const { mock, runner, input } = setup()
    const first = runner.press(input)
    await vi.advanceTimersByTimeAsync(30)
    const rejected = runner.press({ ...input, delta: { x: 0, y: -30 } })
    await vi.advanceTimersByTimeAsync(30)
    await runner.end()

    expect(rejected.contours).toBe(first.contours)
    expect(commandContours(mock)).toEqual([first.contours])
  })

  it('commits the last valid preview even when it was not flushed before rejection', async () => {
    const { mock, runner, input } = setup()
    const first = runner.press(input)
    const rejected = runner.press({ ...input, delta: { x: 0, y: -30 } })
    await runner.end()

    expect(rejected.contours).toBe(first.contours)
    expect(commandContours(mock)).toEqual([first.contours])
  })

  it('carries the accepted projected shape into a new run before reload', async () => {
    const { mock, runner, input } = setup()
    const first = runner.press(input)
    await runner.end()
    const next = { ...input, refs: [{ contour: 0, point: 1 }] }
    const second = runner.press(next)
    await runner.end()

    expect(second.contours).toEqual(
      maintainSuperSmoothContours(
        translatePoints(first.contours, next.refs, 0, 1),
      ),
    )
    expect(commandContours(mock)).toEqual([first.contours, second.contours])
    expect(
      mock.calls.filter((entry) => entry.method === 'doc.edit_begin'),
    ).toHaveLength(2)
  })
})
