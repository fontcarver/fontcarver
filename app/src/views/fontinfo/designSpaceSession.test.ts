import { afterEach, expect, it } from 'vitest'
import { setTransport, type VariationState } from '../../api'
import {
  flushDocumentEditParticipants,
  hasPendingDocumentDraft,
} from '../../editor/documentEditParticipants'
import { makeMaster } from '../../test/fontFixtures'
import { createMockTransport, noChange } from '../../test/mockTransport'
import { DesignSpaceSession } from './designSpaceSession'

afterEach(() => setTransport(undefined))

it('keeps failed metadata drafts and serializes tags correctly on a later successful handover', async () => {
  let value: VariationState = {
    generation: 0,
    axes: [],
    masters: [makeMaster()],
    instances: [],
    first_glyph: 'A',
  }
  let fail = true
  setTransport(
    createMockTransport({
      'variation.state': () => value,
      'variation.apply': (params) => {
        if (fail) throw new Error('stale design space')
        expect(params.expected_generation).toBe(0)
        if ('Set' in params.command) {
          expect(params.command.Set.axes[0]?.tag).toEqual([119, 103, 104, 116])
          value = { ...value, ...params.command.Set, generation: 1 }
        }
        return noChange()
      },
    }),
  )
  const session = new DesignSpaceSession(301, 'Edit design space'),
    dispose = session.connect(() => {})
  try {
    await session.load()
    session.edit({
      ...session.snapshot().value!,
      axes: [
        {
          tag: 'wght',
          name: 'Weight',
          min: 100,
          default: 400,
          max: 900,
          hidden: false,
          map: [],
        },
      ],
    })
    await expect(flushDocumentEditParticipants(301)).rejects.toThrow(
      'stale design space',
    )
    expect(hasPendingDocumentDraft(301)).toBe(true)
    await session.load()
    expect(session.snapshot().value!.axes[0]!.tag).toBe('wght')
    fail = false
    await flushDocumentEditParticipants(301)
    expect(session.snapshot().dirty).toBe(false)
    expect(session.snapshot().value!.generation).toBe(1)
  } finally {
    dispose()
  }
})

it('refuses malformed axis tags before sending a mutation', async () => {
  let writes = 0
  setTransport(
    createMockTransport({
      'variation.state': () => ({
        generation: 0,
        axes: [],
        masters: [makeMaster()],
        instances: [],
        first_glyph: 'A',
      }),
      'variation.apply': () => {
        writes++
        return noChange()
      },
    }),
  )
  const session = new DesignSpaceSession(302, 'Edit')
  await session.load()
  session.edit({
    ...session.snapshot().value!,
    axes: [
      {
        tag: '中',
        name: 'Invalid',
        min: 0,
        default: 0,
        max: 1,
        hidden: false,
        map: [],
      },
    ],
  })
  await expect(session.flush()).rejects.toThrow('four printable ASCII')
  expect(writes).toBe(0)
  expect(session.snapshot().dirty).toBe(true)
})

it('tracks an existing axis through partial tag edits and clears renames after saving', async () => {
  let value: VariationState = {
    generation: 0,
    axes: [
      {
        tag: [119, 103, 104, 116],
        name: 'Weight',
        min: 100,
        default: 400,
        max: 900,
        hidden: false,
        map: [],
      },
    ],
    masters: [makeMaster()],
    instances: [],
    first_glyph: 'A',
  }
  let writes = 0
  setTransport(
    createMockTransport({
      'variation.state': () => value,
      'variation.apply': (params) => {
        if ('Set' in params.command) {
          expect(params.command.Set.axis_renames).toEqual(
            writes++ === 0 ? { wght: 'WGHT' } : {},
          )
          value = {
            ...value,
            ...params.command.Set,
            generation: value.generation + 1,
          }
        }
        return noChange()
      },
    }),
  )
  const session = new DesignSpaceSession(303, 'Edit')
  await session.load()
  for (const tag of ['wgh', '', 'W', 'WGHT']) {
    const draft = session.snapshot().value!
    session.renameAxis(draft.axes[0]!.tag, tag)
    session.edit({ ...draft, axes: [{ ...draft.axes[0]!, tag }] })
  }
  await session.flush()
  session.edit({ ...session.snapshot().value! })
  await session.flush()
  expect(writes).toBe(2)
})

it('flushes virtual master positions with the axis transaction before document save', async () => {
  let value: VariationState = {
    generation: 0,
    axes: [],
    masters: [makeMaster()],
    instances: [],
    first_glyph: 'A',
    virtual_masters: [],
  }
  setTransport(
    createMockTransport({
      'variation.state': () => value,
      'variation.apply': (params) => {
        expect('Set' in params.command).toBe(true)
        if ('Set' in params.command) {
          expect(params.command.Set.virtual_masters).toEqual([
            { name: 'Heavy', location: { wght: 900 } },
          ])
          value = { ...value, ...params.command.Set, generation: 1 }
        }
        return noChange()
      },
    }),
  )
  const session = new DesignSpaceSession(399, 'Virtual masters')
  const dispose = session.connect(() => {})
  try {
    await session.load()
    session.edit({
      ...session.snapshot().value!,
      axes: [
        {
          tag: 'wght',
          name: 'Weight',
          min: 100,
          default: 100,
          max: 900,
          hidden: false,
          map: [],
        },
      ],
      virtual_masters: [{ name: 'Heavy', location: { wght: 900 } }],
    })
    await flushDocumentEditParticipants(399)
    expect(session.snapshot().dirty).toBe(false)
    expect(session.snapshot().value?.virtual_masters).toEqual([
      { name: 'Heavy', location: { wght: 900 } },
    ])
  } finally {
    dispose()
  }
})
