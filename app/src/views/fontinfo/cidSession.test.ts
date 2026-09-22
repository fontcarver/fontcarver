import { afterEach, expect, it } from 'vitest'
import { setTransport, type CidState } from '../../api'
import { flushDocumentEditParticipants } from '../../editor/documentEditParticipants'
import { createMockTransport, noChange } from '../../test/mockTransport'
import {
  CidSession,
  identityMapping,
  parseCid,
  type CidDraft,
} from './cidSession'
const draft: CidDraft = {
  enabled: true,
  registry: 'Adobe',
  ordering: 'Identity',
  supplement: '0',
  mapping: '0 .notdef\n12 A',
  reorder: false,
}
afterEach(() => setTransport(undefined))
it('retains explicit IDs and never silently replaces duplicate mappings', () => {
  const source: CidState = {
    generation: 0,
    cid: null,
    glyphs: ['.notdef', 'A', 'B'],
  }
  expect(identityMapping(source, draft, 'invalid')).toBe(
    '0\t.notdef\n1\tB\n12\tA',
  )
  for (const mapping of ['1 A\n1 B', '1 A\n2 A', '65536 A', '0 A', '2 .notdef'])
    expect(() => parseCid({ ...draft, mapping }, 'invalid')).toThrow('invalid')
  expect(() =>
    identityMapping(source, { ...draft, ordering: 'Japan1' }, 'invalid'),
  ).toThrow('invalid')
})
it('retains failed drafts and joins document-save handovers with a checked revision', async () => {
  let value: CidState = { generation: 4, cid: null, glyphs: ['.notdef', 'A'] },
    fail = true
  setTransport(
    createMockTransport({
      'cid.state': () => value,
      'cid.apply': (params) => {
        expect(params.expected_generation).toBe(4)
        if (fail) throw new Error('stale')
        value = { ...value, cid: params.cid, generation: 5 }
        return noChange()
      },
    }),
  )
  const session = new CidSession(701, 'CID', 'invalid'),
    dispose = session.connect()
  try {
    await session.load()
    session.edit(draft)
    await expect(flushDocumentEditParticipants(701)).rejects.toThrow('stale')
    expect(session.snapshot().dirty).toBe(true)
    fail = false
    await flushDocumentEditParticipants(701)
    expect(session.snapshot().dirty).toBe(false)
    expect(session.snapshot().source?.generation).toBe(5)
    expect(session.snapshot().draft?.mapping).toContain('12\tA')
  } finally {
    dispose()
  }
})
