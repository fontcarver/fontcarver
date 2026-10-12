import { afterEach, describe, expect, it, vi } from 'vitest'
import { setTransport, type BitmapGlyph } from '../../api'
import { createMockTransport, noChange } from '../../test/mockTransport'
import {
  flushDocumentEditParticipants,
  hasPendingDocumentDraft,
} from '../../editor/documentEditParticipants'
import { BitmapSession } from './bitmapSession'
import { blank, put } from './pixels'
const empty: BitmapGlyph = {
  bitmap: blank(8, 8, 1),
  advance: 8,
  vertical_advance: null,
  refs: [],
}
afterEach(() => setTransport(undefined))
describe('bitmap document handover', () => {
  it.each(['cancel', 'finish'])(
    'defers clean-selection reload until %s, retaining coherent pixels and generation',
    async (action) => {
      let value = empty,
        generation = 1
      const transport = createMockTransport({
        'bitmap.glyph': () => ({ generation, value, resolved: value }),
      })
      setTransport(transport)
      const session = new BitmapSession(
        { doc: 212, glyph: 'A', ppem: 16, depth: 1 },
        empty,
      )
      await session.load()
      session.beginGesture()
      value = { ...empty, advance: 99 }
      generation = 2
      await session.load()
      expect(transport.calls).toHaveLength(1)
      expect(session.snapshot().busy).toBe(false)
      expect(session.snapshot().generation).toBe(1)
      if (action === 'cancel') session.cancelGesture()
      else session.endGesture()
      await vi.waitFor(() => expect(session.snapshot().busy).toBe(false))
      expect(session.snapshot().value?.advance).toBe(99)
      expect(session.snapshot().generation).toBe(2)
      expect(session.snapshot().dirty).toBe(false)
    },
  )
  it('cancelling a gesture restores the clean draft without creating a later write', async () => {
    const transport = createMockTransport({
      'bitmap.glyph': () => ({ generation: 1, value: empty, resolved: empty }),
      'bitmap.apply': () => noChange(),
    })
    setTransport(transport)
    const session = new BitmapSession(
      { doc: 209, glyph: 'A', ppem: 16, depth: 1 },
      empty,
    )
    const dispose = session.connect()
    await session.load()
    session.beginGesture()
    const edited = structuredClone(empty)
    put(edited.bitmap, 1, 1, 1)
    session.edit(edited)
    session.cancelGesture()
    expect(session.snapshot().value).toEqual(empty)
    expect(session.snapshot().resolved).toEqual(empty)
    expect(hasPendingDocumentDraft(209)).toBe(false)
    await flushDocumentEditParticipants(209)
    expect(
      transport.calls.filter((call) => call.method === 'bitmap.apply'),
    ).toHaveLength(0)
    dispose()
  })
  it('cancelling a new stroke preserves a previous failed-write draft and error', async () => {
    setTransport(
      createMockTransport({
        'bitmap.glyph': () => ({
          generation: 1,
          value: empty,
          resolved: empty,
        }),
        'bitmap.apply': () => {
          throw new Error('write rejected')
        },
      }),
    )
    const session = new BitmapSession(
      { doc: 210, glyph: 'A', ppem: 16, depth: 1 },
      empty,
    )
    await session.load()
    const previous = { ...empty, advance: 11 }
    session.edit(previous)
    await expect(session.flush()).rejects.toThrow('write rejected')
    const error = session.snapshot().error
    session.beginGesture()
    session.edit({ ...previous, advance: 12 })
    session.cancelGesture()
    expect(session.snapshot().value).toEqual(previous)
    expect(session.snapshot().dirty).toBe(true)
    expect(session.snapshot().error).toBe(error)
  })
  it('cancellation after a save barrier cannot undo an accepted bitmap draft', async () => {
    let value = empty
    let release!: () => void
    const pending = new Promise<void>((resolve) => {
      release = resolve
    })
    setTransport(
      createMockTransport({
        'bitmap.glyph': () => ({ generation: 1, value, resolved: value }),
        'bitmap.apply': async ({ command }) => {
          if ('SetGlyph' in command) value = command.SetGlyph.value!
          await pending
          return noChange()
        },
      }),
    )
    const session = new BitmapSession(
      { doc: 211, glyph: 'A', ppem: 16, depth: 1 },
      empty,
    )
    await session.load()
    session.beginGesture()
    const edited = { ...empty, advance: 12 }
    session.edit(edited)
    const saving = session.flush()
    session.cancelGesture()
    expect(session.snapshot().value).toEqual(edited)
    release()
    await saving
    session.cancelGesture()
    expect(session.snapshot().value).toEqual(edited)
    expect(session.snapshot().dirty).toBe(false)
  })
  it('keeps editable references separate from composed display pixels', async () => {
    let source = {
      ...structuredClone(empty),
      refs: [{ base: 'acute', dx: 2, dy: 5 }],
    }
    const resolved = structuredClone(empty)
    put(resolved.bitmap, 3, 1, 1)
    setTransport(
      createMockTransport({
        'bitmap.glyph': () => ({ generation: 1, value: source, resolved }),
        'bitmap.apply': ({ command }) => {
          if ('SetGlyph' in command) source = command.SetGlyph.value!
          return noChange()
        },
      }),
    )
    const session = new BitmapSession(
      { doc: 203, glyph: 'Aacute', ppem: 16, depth: 1 },
      empty,
    )
    await session.load()
    expect(session.snapshot().value).toEqual(source)
    expect(session.snapshot().resolved).toEqual(resolved)
    session.edit({ ...source, advance: 9 })
    expect(session.snapshot().resolved).toBeNull()
    await session.flush()
    expect(source.refs).toEqual([{ base: 'acute', dx: 2, dy: 5 }])
    expect(source.bitmap.rows.every((pixel) => pixel === 0)).toBe(true)
    expect(session.snapshot().resolved).toEqual(resolved)
  })
  it('persists one completed draft before navigation and retains failed pixels', async () => {
    let fail = true,
      generation = 0,
      value = structuredClone(empty)
    setTransport(
      createMockTransport({
        'bitmap.glyph': () => ({ generation, value, resolved: value }),
        'bitmap.apply': (p) => {
          if (fail) throw new Error('disk unavailable')
          if ('SetGlyph' in p.command) value = p.command.SetGlyph.value!
          generation++
          return noChange()
        },
      }),
    )
    const session = new BitmapSession(
        { doc: 201, glyph: 'A', ppem: 16, depth: 1 },
        empty,
      ),
      dispose = session.connect()
    await session.load()
    const edited = structuredClone(empty)
    put(edited.bitmap, 3, 4, 1)
    session.edit(edited)
    expect(hasPendingDocumentDraft(201)).toBe(true)
    await expect(flushDocumentEditParticipants(201)).rejects.toThrow(
      'disk unavailable',
    )
    expect(session.snapshot().dirty).toBe(true)
    expect(session.snapshot().value).toEqual(edited)
    await session.load()
    expect(session.snapshot().value).toEqual(edited)
    fail = false
    await flushDocumentEditParticipants(201)
    expect(value).toEqual(edited)
    expect(session.snapshot().dirty).toBe(false)
    dispose()
  })
  it('an edit made during a background refresh supersedes it instead of being dropped', async () => {
    // A document event reloads the draft right after every write. A click that
    // lands during that reload must still become an edit, and the reload's
    // stale result must not overwrite it.
    let release!: () => void
    const held = new Promise<void>((resolve) => {
      release = resolve
    })
    const applied: unknown[] = []
    let hold = false
    setTransport(
      createMockTransport({
        'bitmap.glyph': async () => {
          if (hold) await held
          return { generation: 1, value: empty, resolved: empty }
        },
        'bitmap.apply': (params) => {
          applied.push(params)
          return noChange()
        },
      }),
    )
    const session = new BitmapSession(
      { doc: 213, glyph: 'A', ppem: 16, depth: 1 },
      empty,
    )
    await session.load()
    hold = true
    const refresh = session.load()
    expect(session.snapshot().busy).toBe(true)
    session.edit({ ...empty, advance: 99 })
    expect(session.snapshot().value?.advance).toBe(99)
    expect(session.snapshot().dirty).toBe(true)
    expect(session.snapshot().busy).toBe(false)
    release()
    await refresh
    expect(session.snapshot().value?.advance).toBe(99)
    hold = false
    await session.flush()
    expect(applied).toHaveLength(1)
  })
  it('reports saving only while a write is in flight, not during a refresh', async () => {
    // The editor disables its tools on `saving`. A refresh follows every write;
    // if it disabled the buttons too, a click landing then would reach a
    // disabled button and the browser would drop it before any handler ran.
    let releaseLoad!: () => void
    let releaseWrite!: () => void
    const loadHeld = new Promise<void>((resolve) => {
      releaseLoad = resolve
    })
    const writeHeld = new Promise<void>((resolve) => {
      releaseWrite = resolve
    })
    let hold = false
    setTransport(
      createMockTransport({
        'bitmap.glyph': async () => {
          if (hold) await loadHeld
          return { generation: 1, value: empty, resolved: empty }
        },
        'bitmap.apply': async () => {
          await writeHeld
          return noChange()
        },
      }),
    )
    const session = new BitmapSession(
      { doc: 214, glyph: 'A', ppem: 16, depth: 1 },
      empty,
    )
    await session.load()
    hold = true
    const refresh = session.load()
    expect(session.snapshot().busy).toBe(true)
    expect(session.snapshot().saving).toBe(false)
    releaseLoad()
    await refresh
    hold = false
    session.edit({ ...empty, advance: 99 })
    const write = session.flush()
    expect(session.snapshot().saving).toBe(true)
    releaseWrite()
    await write
    expect(session.snapshot().saving).toBe(false)
  })
  it('joins simultaneous flushes and rejects local input while saving', async () => {
    let release!: () => void
    const held = new Promise<void>((resolve) => {
      release = resolve
    })
    let writes = 0
    setTransport(
      createMockTransport({
        'bitmap.glyph': () => ({
          generation: writes,
          value: empty,
          resolved: empty,
        }),
        'bitmap.apply': async () => {
          writes++
          await held
          return noChange()
        },
      }),
    )
    const session = new BitmapSession(
      { doc: 202, glyph: 'A', ppem: 16, depth: 1 },
      empty,
    )
    await session.load()
    session.edit(empty)
    const a = session.flush(),
      b = session.flush()
    expect(a).toBe(b)
    session.edit({ ...empty, advance: 99 })
    expect(session.snapshot().value?.advance).toBe(8)
    release()
    await a
    expect(writes).toBe(1)
  })
})
