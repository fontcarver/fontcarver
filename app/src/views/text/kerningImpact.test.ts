import { afterEach, expect, it } from 'vitest'
import { setTransport, type KerningResolution } from '../../api'
import { createMockTransport } from '../../test/mockTransport'
import { loadKerningImpact } from './kerningImpact'

afterEach(() => setTransport(undefined))

it('reads a bounded member preview at one generation for both group sides', async () => {
  const transport = createMockTransport({
    'kerning.members': ({ side }) => ({
      status: { editable: true, reason: null, generation: 7 },
      total: 320,
      items: side === 'side1' ? ['A', 'a'] : ['V', 'v'],
    }),
  })
  setTransport(transport)
  const resolution = {
    status: { editable: true, reason: null, generation: 7 },
    left_group: 'public.kern1.A',
    right_group: 'public.kern2.V',
    left: 'A',
    right: 'V',
  } as KerningResolution
  const impact = await loadKerningImpact(1, resolution, 'group_group')
  expect(transport.calls.map((item) => item.params)).toEqual([
    {
      doc: 1,
      side: 'side1',
      name: 'public.kern1.A',
      generation: 7,
      offset: 0,
      limit: 8,
    },
    {
      doc: 1,
      side: 'side2',
      name: 'public.kern2.V',
      generation: 7,
      offset: 0,
      limit: 8,
    },
  ])
  expect(impact).toEqual([
    { name: 'public.kern1.A', total: 320, members: ['A', 'a'] },
    { name: 'public.kern2.V', total: 320, members: ['V', 'v'] },
  ])
})

it('needs no group member transfer for a concrete glyph pair', async () => {
  const transport = createMockTransport({})
  setTransport(transport)
  expect(
    await loadKerningImpact(
      1,
      { left: 'A', right: 'V' } as KerningResolution,
      'glyph_glyph',
    ),
  ).toEqual([])
  expect(transport.calls).toHaveLength(0)
})
