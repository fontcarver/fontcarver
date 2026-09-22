import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { setTransport, type Command, type EditApplyParams } from '../../api'
import {
  createMockTransport,
  type MockTransport,
} from '../../test/mockTransport'
import { immediateScheduler } from '../editSession'

import { NO_MODIFIERS } from './drag'
import { NudgeCoalescer, NUDGE_TIMEOUT, nudgeStep } from './nudge'

const refs = [{ contour: 0, point: 1 }]

function transport(): MockTransport {
  const mock = createMockTransport({
    'doc.edit_begin': () => 3,
    'doc.edit_apply': () => ({
      glyphs: [1],
      font_info: false,
      masters: false,
      layout: false,
      bitmaps: false,
      feature_source: false,
      glyph_order: false,
      metrics: false,
      layers: false,
      guides: false,
    }),
    'doc.edit_end': () => null,
  })
  setTransport(mock)
  return mock
}

function coalescer(): NudgeCoalescer {
  return new NudgeCoalescer({
    doc: 1,
    glyph: 'a',
    layer: 'layer-1',
    label: (count) => `Nudge ${count} points`,
    scheduler: immediateScheduler,
  })
}

function deltas(mock: MockTransport): Command[] {
  return mock.calls
    .filter((entry) => entry.method === 'doc.edit_apply')
    .map((entry) => (entry.params as EditApplyParams).command)
}

beforeEach(() => vi.useFakeTimers())

afterEach(() => {
  vi.useRealTimers()
  setTransport(undefined)
})

describe('nudgeStep', () => {
  it('is 1, 10 with Shift and 100 with Ctrl', () => {
    expect(nudgeStep(NO_MODIFIERS)).toBe(1)
    expect(nudgeStep({ ...NO_MODIFIERS, shift: true })).toBe(10)
    expect(nudgeStep({ ...NO_MODIFIERS, mod: true })).toBe(100)
  })
})

describe('NudgeCoalescer', () => {
  it('coalesces a run of presses into one undo entry', async () => {
    const mock = transport()
    const nudge = coalescer()

    nudge.nudge(refs, { x: -1, y: 0 }, 'WithHandles')
    await nudge.flush()
    vi.advanceTimersByTime(200)
    nudge.nudge(refs, { x: -1, y: 0 }, 'WithHandles')
    await nudge.flush()
    vi.advanceTimersByTime(200)
    nudge.nudge(refs, { x: -10, y: 0 }, 'WithHandles')
    await nudge.flush()

    expect(
      mock.calls.filter((entry) => entry.method === 'doc.edit_begin'),
    ).toHaveLength(1)
    expect(deltas(mock)).toHaveLength(3)
    expect(
      mock.calls.filter((entry) => entry.method === 'doc.edit_end'),
    ).toHaveLength(0)

    vi.advanceTimersByTime(NUDGE_TIMEOUT)
    await vi.runAllTimersAsync()
    expect(
      mock.calls.filter((entry) => entry.method === 'doc.edit_end'),
    ).toHaveLength(1)
    expect(nudge.active).toBe(false)
  })

  it('starts a new entry after 500 ms of silence', async () => {
    const mock = transport()
    const nudge = coalescer()

    nudge.nudge(refs, { x: 0, y: 1 }, 'WithHandles')
    await nudge.flush()
    vi.advanceTimersByTime(NUDGE_TIMEOUT + 1)
    await vi.runAllTimersAsync()
    nudge.nudge(refs, { x: 0, y: 1 }, 'WithHandles')
    await nudge.flush()
    await nudge.end()

    expect(
      mock.calls.filter((entry) => entry.method === 'doc.edit_begin'),
    ).toHaveLength(2)
  })

  it('holds exactly one run, whatever it is handed', async () => {
    // Replacing a run when the selection changes is the runner's job — it also
    // has to carry the closed run's geometry forward and gate the next
    // `edit_begin`, neither of which the coalescer can see. One coalescer is
    // one run, so nothing here has to notice that the set moved.
    const mock = transport()
    const nudge = coalescer()

    nudge.nudge(refs, { x: 1, y: 0 }, 'WithHandles')
    await nudge.flush()
    nudge.nudge([{ contour: 0, point: 2 }], { x: 1, y: 0 }, 'WithHandles')
    await nudge.flush()
    await nudge.end()

    expect(
      mock.calls.filter((entry) => entry.method === 'doc.edit_begin'),
    ).toHaveLength(1)
    expect(
      mock.calls.filter((entry) => entry.method === 'doc.edit_end'),
    ).toHaveLength(1)
  })

  it('sends the step since the previous apply, which sums to the run', async () => {
    const mock = transport()
    const nudge = coalescer()

    nudge.nudge(refs, { x: 0, y: -1 }, 'PointsOnly')
    await nudge.flush()
    nudge.nudge(refs, { x: 0, y: -1 }, 'PointsOnly')
    await nudge.flush()
    await nudge.end()

    expect(deltas(mock)).toEqual([
      {
        MovePoints: {
          glyph: 'a',
          layer: 'layer-1',
          points: refs,
          delta: [0, -1],
          handles: 'PointsOnly',
        },
      },
      {
        MovePoints: {
          glyph: 'a',
          layer: 'layer-1',
          points: refs,
          delta: [0, -1],
          handles: 'PointsOnly',
        },
      },
    ])
  })
})
