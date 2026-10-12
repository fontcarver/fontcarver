import { afterEach, expect, it } from 'vitest'
import { setTransport, type GlyphCell } from '../../api'
import { makeCell } from '../../test/fontFixtures'
import { createMockTransport } from '../../test/mockTransport'
import { ghostMembers, loadKerningGhostCells } from './kerningGhosts'
import type { TextKerningModel } from './kerningModel'

afterEach(() => setTransport(undefined))

it('bounds alternate member outlines per side and omits the actual pair', () => {
  const model = {
    scope: 'group_group',
    pair: { left: 'A', right: 'V' },
    resolution: { left_group: 'A-group', right_group: 'V-group' },
    impact: [
      {
        name: 'A-group',
        members: ['A', 'a', 'b', 'c', 'd', 'e', 'a'],
        total: 20,
      },
      { name: 'V-group', members: ['V', 'v', 'w', 'x', 'y', 'z'], total: 20 },
    ],
  } as TextKerningModel
  expect(ghostMembers(model)).toEqual([
    { name: 'a', side: 0 },
    { name: 'b', side: 0 },
    { name: 'c', side: 0 },
    { name: 'd', side: 0 },
    { name: 'v', side: 1 },
    { name: 'w', side: 1 },
    { name: 'x', side: 1 },
    { name: 'y', side: 1 },
  ])
  expect(ghostMembers({ ...model, scope: 'glyph_glyph' })).toEqual([])
})

it('requires exact names and active-master paths, without requesting every member', async () => {
  const transport = createMockTransport({
    'glyphs.search': ({ query }) => [
      makeCell({ gid: 10, name: `${query}.alt` }),
      ...(query === 'a' ? [makeCell({ gid: 11, name: query })] : []),
    ],
    'glyphs.cell': ({ gid, master }) =>
      makeCell({
        gid,
        name: 'a',
        path: `M0 0L${master === 'bold' ? 9 : 1} 9Z`,
      }),
  })
  setTransport(transport)
  const result = await loadKerningGhostCells(
    1,
    'bold',
    ['a', 'missing', 'a'],
    new AbortController().signal,
  )
  expect([...result.keys()]).toEqual(['a'])
  expect(result.get('a')?.path).toBe('M0 0L9 9Z')
  expect(transport.calls.map(({ method }) => method)).toEqual([
    'glyphs.search',
    'glyphs.search',
    'glyphs.cell',
  ])
  expect(transport.calls[2]?.params).toEqual({
    doc: 1,
    master: 'bold',
    gid: 11,
  })
})

it('aborts queued member work before loading cells from an obsolete response', async () => {
  let release: ((value: GlyphCell[]) => void) | undefined
  const transport = createMockTransport({
    'glyphs.search': () =>
      new Promise<GlyphCell[]>((resolve) => {
        release = resolve
      }),
  })
  setTransport(transport)
  const abort = new AbortController()
  const loading = loadKerningGhostCells(1, 'regular', ['a'], abort.signal)
  abort.abort()
  release?.([makeCell({ name: 'a' })])
  await expect(loading).rejects.toMatchObject({ name: 'AbortError' })
  expect(transport.calls.map(({ method }) => method)).toEqual(['glyphs.search'])
})

it('keeps the two sides distinct when their group names are identical', () => {
  const model = {
    scope: 'group_group',
    pair: { left: 'A', right: 'V' },
    resolution: { left_group: 'shared', right_group: 'shared' },
    impact: [
      { name: 'shared', members: ['A', 'a'], total: 2 },
      { name: 'shared', members: ['V', 'v'], total: 2 },
    ],
  } as TextKerningModel
  expect(ghostMembers(model)).toEqual([
    { name: 'a', side: 0 },
    { name: 'v', side: 1 },
  ])
  expect(
    ghostMembers({
      ...model,
      scope: 'glyph_group',
      impact: [model.impact[1]!],
    }),
  ).toEqual([{ name: 'v', side: 1 }])
})
