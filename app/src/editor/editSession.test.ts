import { afterEach, describe, expect, it, vi } from 'vitest'

import { setTransport } from '../api'
import {
  createEditingTransport,
  createMockTransport,
  noChange,
} from '../test/mockTransport'

import { EditSession, immediateScheduler } from './editSession'

const move = () => ({
  MovePoints: {
    glyph: 'a',
    layer: 'layer-1',
    points: [{ contour: 0, point: 1 }],
    delta: [1, 0] as [number, number],
    handles: 'PointsOnly' as const,
  },
})

afterEach(() => {
  setTransport(undefined)
  vi.restoreAllMocks()
})

describe('EditSession failures', () => {
  it('reports a failing edit_begin exactly once for the whole gesture', async () => {
    setTransport(
      createEditingTransport({
        failBegin: () => new Error('document 1 is closed'),
      }),
    )
    const errors: unknown[] = []
    const session = new EditSession(1, 'Move 1 point', {
      scheduler: immediateScheduler,
      onError: (error) => errors.push(error),
    })

    session.queue(() => move())
    await session.flush()
    await session.end()

    // Every later step re-awaits the same rejected token; one dead document is
    // one toast, not one per queued call.
    expect(errors).toHaveLength(1)
    expect(String(errors[0])).toContain('document 1 is closed')
    expect(session.failed).toBe(true)
    expect(session.dirty).toBe(false)
  })

  it('warns rather than reports a second, unrelated failure', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined)
    setTransport(
      createMockTransport({
        'doc.edit_begin': () => 7,
        'doc.edit_apply': () => {
          throw new Error('apply refused')
        },
        'doc.edit_end': () => {
          throw new Error('end refused')
        },
      }),
    )
    const errors: unknown[] = []
    const session = new EditSession(1, 'Move 1 point', {
      scheduler: immediateScheduler,
      onError: (error) => errors.push(error),
    })

    session.queue(() => move())
    await session.flush()
    await session.end()

    expect(errors.map(String)).toEqual(['Error: apply refused'])
    // The unrelated failure still leaves a trace, in the console rather than
    // in a second toast on top of the first.
    expect(warn).toHaveBeenCalledTimes(1)
    expect(String(warn.mock.calls[0]?.[1])).toContain('end refused')
  })

  it('reports nothing when every call succeeds', async () => {
    setTransport(
      createMockTransport({
        'doc.edit_begin': () => 7,
        'doc.edit_apply': () => noChange([42]),
        'doc.edit_end': () => null,
      }),
    )
    const errors: unknown[] = []
    const session = new EditSession(1, 'Move 1 point', {
      scheduler: immediateScheduler,
      onError: (error) => errors.push(error),
    })
    session.queue(() => move())
    await session.end()

    expect(errors).toEqual([])
    expect(session.failed).toBe(false)
    expect(session.dirty).toBe(true)
  })
})
