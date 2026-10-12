import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { useSyncExternalStore } from 'react'
import { afterEach, describe, expect, it } from 'vitest'
import { setTransport, type BitmapGlyph } from '../../api'
import { flushDocumentEditParticipants } from '../../editor/documentEditParticipants'
import { createMockTransport, noChange } from '../../test/mockTransport'
import { BitmapReferences } from './BitmapReferences'
import { BitmapSession } from './bitmapSession'
import { blank, put } from './pixels'

afterEach(() => setTransport(undefined))

function Harness({ session }: { session: BitmapSession }) {
  const state = useSyncExternalStore(session.subscribe, session.snapshot)
  return (
    <BitmapReferences
      session={session}
      value={state.value!}
      disabled={state.busy || state.suspended}
    />
  )
}

async function setup(refs: BitmapGlyph['refs'] = []) {
  let source: BitmapGlyph = {
    bitmap: blank(8, 8, 1),
    advance: 9,
    vertical_advance: 12,
    vertical_origin: [4, 7],
    refs,
  }
  const composed = blank(10, 12, 1)
  composed.origin_y = -2
  put(composed, 2, 1, 1)
  let generation = 0
  let failWrite = false
  const transport = createMockTransport({
    'bitmap.glyph': () => ({
      generation,
      value: structuredClone(source),
      resolved: { ...structuredClone(source), bitmap: composed, refs: [] },
    }),
    'bitmap.apply': ({ command }) => {
      if (failWrite) throw new Error('Bitmap write failed')
      if ('SetGlyph' in command) source = command.SetGlyph.value!
      generation++
      return noChange()
    },
  })
  setTransport(transport)
  const session = new BitmapSession(
    { doc: 301, glyph: 'Aacute', ppem: 16, depth: 1 },
    source,
  )
  await session.load()
  render(<Harness session={session} />)
  return {
    session,
    transport,
    source: () => source,
    composed,
    failWrite: (fail: boolean) => {
      failWrite = fail
    },
  }
}

describe('bitmap reference controls', () => {
  it('adds translated components without replacing source pixels or metrics', async () => {
    const { source } = await setup()
    fireEvent.change(
      screen.getByRole('textbox', { name: 'Referenced glyph' }),
      {
        target: { value: 'acute' },
      },
    )
    fireEvent.change(
      screen.getByRole('spinbutton', { name: 'Horizontal offset' }),
      {
        target: { value: '3' },
      },
    )
    fireEvent.change(
      screen.getByRole('spinbutton', { name: 'Vertical offset' }),
      {
        target: { value: '6' },
      },
    )
    fireEvent.click(screen.getByRole('button', { name: 'Add component' }))
    await waitFor(() =>
      expect(source().refs).toEqual([{ base: 'acute', dx: 3, dy: 6 }]),
    )
    expect(source().bitmap).toEqual(blank(8, 8, 1))
    expect(source().vertical_origin).toEqual([4, 7])
  })

  it('persists a focused offset draft through the document save barrier', async () => {
    const { source } = await setup([{ base: 'acute', dx: 0, dy: 0 }])
    fireEvent.change(
      screen.getByRole('spinbutton', { name: 'acute: Horizontal offset' }),
      {
        target: { value: '-4' },
      },
    )
    await act(() => flushDocumentEditParticipants(301))
    expect(source().refs).toEqual([{ base: 'acute', dx: -4, dy: 0 }])
  })

  it('removes one component and flattens the remaining composed pixels', async () => {
    const { source, composed } = await setup([
      { base: 'A', dx: 0, dy: 0 },
      { base: 'acute', dx: 1, dy: 5 },
    ])
    fireEvent.click(screen.getByRole('button', { name: 'Remove A component' }))
    await waitFor(() => expect(source().refs).toHaveLength(1))
    await waitFor(() =>
      expect(
        screen.getByRole('button', { name: 'Flatten components' }),
      ).toBeEnabled(),
    )
    fireEvent.click(screen.getByRole('button', { name: 'Flatten components' }))
    await waitFor(() => expect(source().refs).toEqual([]))
    expect(source().bitmap).toEqual(composed)
    expect(source().advance).toBe(9)
    expect(source().vertical_origin).toEqual([4, 7])
  })

  it('rejects invalid offset drafts without clearing the pending edit', async () => {
    const { source } = await setup([{ base: 'acute', dx: 0, dy: 0 }])
    fireEvent.change(
      screen.getByRole('spinbutton', { name: 'acute: Vertical offset' }),
      {
        target: { value: '40000' },
      },
    )
    await expect(flushDocumentEditParticipants(301)).rejects.toThrow(
      'Component offsets',
    )
    expect(source().refs[0].dy).toBe(0)
    expect(
      screen.getByRole('spinbutton', { name: 'acute: Vertical offset' }),
    ).toHaveValue(40000)
  })

  it('retains a component draft after a failed write and permits retry', async () => {
    const context = await setup()
    context.failWrite(true)
    fireEvent.change(
      screen.getByRole('textbox', { name: 'Referenced glyph' }),
      {
        target: { value: 'acute' },
      },
    )
    fireEvent.click(screen.getByRole('button', { name: 'Add component' }))
    await waitFor(() =>
      expect(context.session.snapshot().error).toBeInstanceOf(Error),
    )
    expect(context.session.snapshot().dirty).toBe(true)
    expect(context.session.snapshot().value?.refs).toEqual([
      { base: 'acute', dx: 0, dy: 0 },
    ])
    expect(context.source().refs).toEqual([])
    context.failWrite(false)
    await act(() => context.session.flush())
    expect(context.source().refs).toEqual([{ base: 'acute', dx: 0, dy: 0 }])
    expect(context.session.snapshot().dirty).toBe(false)
  })
})
