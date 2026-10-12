import { afterEach, describe, expect, it, vi } from 'vitest'

import { setTransport } from '../api'
import {
  createEditingTransport,
  createMockTransport,
  noChange,
} from '../test/mockTransport'

import {
  applyCommand,
  EditSession,
  holdDocumentEdits,
  immediateScheduler,
  documentEditRevision,
  noteDocumentEdit,
} from './editSession'

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

describe('document edit handover', () => {
  it('counts staged drafts and new sessions only for their own document', async () => {
    setTransport(createEditingTransport())
    const before = documentEditRevision(1)
    const other = documentEditRevision(2)
    noteDocumentEdit(1)
    expect(documentEditRevision(1)).toBe(before + 1)
    const session = new EditSession(1, 'Kerning')
    expect(documentEditRevision(1)).toBe(before + 2)
    expect(documentEditRevision(2)).toBe(other)
    await session.end()
    expect(documentEditRevision(1)).toBe(before + 2)
  })

  it('keeps documents independent while one handover is blocked', async () => {
    let release = () => undefined as void
    holdDocumentEdits(
      1,
      new Promise<void>((resolve) => {
        release = resolve
      }),
    )
    const transport = createMockTransport({
      'doc.apply': () => noChange([42]),
    })
    setTransport(transport)

    const blocked = applyCommand(1, move(), 'Move point')
    await applyCommand(2, move(), 'Move point')
    expect(transport.calls.map((entry) => entry.params)).toEqual([
      expect.objectContaining({ doc: 2 }),
    ])

    release()
    await blocked
    expect(transport.calls.map((entry) => entry.params)).toEqual([
      expect.objectContaining({ doc: 2 }),
      expect.objectContaining({ doc: 1 }),
    ])
  })
})

describe('token-scoped cancellation', () => {
  it('never undoes a newer gesture when an older session is cancelled', async () => {
    const transport = createEditingTransport()
    setTransport(transport)
    const old = new EditSession(91, 'Old', { scheduler: immediateScheduler })
    old.queue(move)
    await old.flush()
    const current = new EditSession(91, 'New', {
      scheduler: immediateScheduler,
    })
    current.queue(move)
    await current.flush()
    await old.cancel()
    expect(old.failed).toBe(true)
    current.queue(move)
    await current.flush()
    expect(current.failed).toBe(false)
    await current.end()
    expect(transport.calls.some((entry) => entry.method === 'doc.undo')).toBe(
      false,
    )
  })

  it('cannot cancel an already committed gesture', async () => {
    const transport = createEditingTransport()
    setTransport(transport)
    const session = new EditSession(92, 'Finished', {
      scheduler: immediateScheduler,
    })
    session.queue(move)
    await session.end()
    const before = transport.calls.length
    await session.cancel()
    expect(transport.calls).toHaveLength(before)
  })

  it('waits for an in-flight apply before atomically cancelling it', async () => {
    let release!: () => void
    const gate = new Promise<void>((resolve) => {
      release = resolve
    })
    const transport = createEditingTransport({
      hold: { on: 'doc.edit_apply', until: gate },
    })
    setTransport(transport)
    const session = new EditSession(93, 'Pending', {
      scheduler: immediateScheduler,
    })
    session.queue(move)
    await session.ready()
    const cancelled = session.cancel()
    expect(
      transport.calls.some((entry) => entry.method === 'doc.edit_cancel'),
    ).toBe(false)
    release()
    await cancelled
    expect(transport.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_cancel',
    ])
  })
})
