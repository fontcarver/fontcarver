import { afterEach, describe, expect, it } from 'vitest'
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
