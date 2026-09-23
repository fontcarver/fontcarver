import { afterEach, expect, it } from 'vitest'
import { setTransport, type MathState, type MathTable } from '../../api'
import { flushDocumentEditParticipants } from '../../editor/documentEditParticipants'
import { createMockTransport, noChange } from '../../test/mockTransport'
import {
  CONSTANT_GROUPS,
  diffCommands,
  emptyConstants,
  emptyGlyph,
  emptyTable,
  formatDevice,
  parseDevice,
} from './mathModel'
import { MathSession } from './mathSession'

afterEach(() => setTransport(undefined))

it('covers all 51 value constants and the plain constants exactly once', () => {
  const values = Object.values(CONSTANT_GROUPS).flat()
  expect(values).toHaveLength(51)
  expect(new Set(values).size).toBe(51)
  expect(Object.keys(emptyConstants())).toHaveLength(56)
})

it('parses device corrections strictly and formats them back', () => {
  expect(parseDevice('')).toBeNull()
  expect(parseDevice(' 12: -2 -1 0 1 ')).toEqual({
    start_size: 12,
    deltas: [-2, -1, 0, 1],
  })
  expect(parseDevice('9: 1, 2')).toEqual({ start_size: 9, deltas: [1, 2] })
  for (const invalid of [
    '12',
    '12:',
    'a: 1',
    '12: 128',
    '12: -129',
    '65535: 1 2',
    '1: 1.5',
  ])
    expect(parseDevice(invalid)).toBeUndefined()
  expect(formatDevice({ start_size: 12, deltas: [-1, 0] })).toBe('12: -1 0')
  expect(formatDevice(null)).toBe('')
})

it('sends whole-table replacement only for creation and deletion', () => {
  const table = emptyTable()
  expect(diffCommands(null, null)).toEqual([])
  expect(diffCommands(null, table)).toEqual([{ Replace: { math: table } }])
  expect(diffCommands(table, null)).toEqual([{ Replace: { math: null } }])
  const edited: MathTable = {
    ...table,
    min_connector_overlap: 5,
    glyphs: { A: { ...emptyGlyph(), extended_shape: true } },
  }
  const source: MathTable = { ...table, glyphs: { B: emptyGlyph() } }
  expect(diffCommands(source, edited)).toEqual([
    {
      SetGlobals: {
        constants: table.constants,
        min_connector_overlap: 5,
      },
    },
    { SetGlyph: { glyph: 'A', info: edited.glyphs.A } },
    { SetGlyph: { glyph: 'B', info: null } },
  ])
  expect(diffCommands(edited, edited)).toEqual([])
})

it('retains failed drafts and applies revision-checked diffs on handover', async () => {
  let value: MathState = {
    generation: 3,
    status: 'Absent',
    math: null,
    import_error: null,
    units_per_em: 1000,
  }
  let fail = true
  const applied: unknown[] = []
  setTransport(
    createMockTransport({
      'math.state': () => value,
      'math.apply': (params) => {
        expect(params.expected_generation).toBe(3)
        if (fail) throw new Error('stale')
        applied.push(params.commands)
        value = {
          ...value,
          status: 'Editable',
          math: emptyTable(),
          generation: 4,
        }
        return noChange()
      },
    }),
  )
  const session = new MathSession(802, { edit: 'MATH', decode: 'Decode' })
  const dispose = session.connect()
  try {
    await session.load()
    session.edit(emptyTable())
    await expect(flushDocumentEditParticipants(802)).rejects.toThrow('stale')
    expect(session.snapshot().dirty).toBe(true)
    expect(session.snapshot().draft).toEqual(emptyTable())
    fail = false
    await flushDocumentEditParticipants(802)
    expect(applied).toEqual([[{ Replace: { math: emptyTable() } }]])
    expect(session.snapshot().dirty).toBe(false)
    expect(session.snapshot().source?.generation).toBe(4)
    // A no-op draft clears without a request.
    session.edit(emptyTable())
    await session.flush()
    expect(applied).toHaveLength(1)
    expect(session.snapshot().dirty).toBe(false)
  } finally {
    dispose()
  }
})

it('decodes imported bytes with the current revision', async () => {
  let value: MathState = {
    generation: 7,
    status: 'Imported',
    math: null,
    import_error: null,
    units_per_em: 2048,
  }
  setTransport(
    createMockTransport({
      'math.state': () => value,
      'math.decode_imported': (params) => {
        expect(params).toEqual({ doc: 5, expected_generation: 7 })
        value = {
          ...value,
          status: 'Editable',
          math: emptyTable(),
          generation: 8,
        }
        return noChange()
      },
    }),
  )
  const session = new MathSession(5, { edit: 'MATH', decode: 'Decode' })
  await session.load()
  await session.decode()
  expect(session.snapshot().source?.status).toBe('Editable')
  expect(session.snapshot().draft).toEqual(emptyTable())
})
