import {
  act,
  render,
  renderHook,
  screen,
  waitFor,
} from '@testing-library/react'
import { afterEach, expect, it, vi } from 'vitest'
import userEvent from '@testing-library/user-event'
import { setTransport, type GlyphCell } from '../../api'
import { makeCell } from '../../test/fontFixtures'
import { createMockTransport } from '../../test/mockTransport'
import { KerningGhostControls } from './KerningGhostControls'
import type { TextKerningModel } from './kerningModel'
import type { PositionedGlyph } from './textLayout'
import { useKerningGhosts } from './useKerningGhosts'

const model = {
  pair: { left: 'A', right: 'V' },
  scope: 'group_group',
  loading: false,
  resolution: {
    left: 'A',
    right: 'V',
    status: { generation: 7 },
    value: -40,
    left_group: 'A-group',
    right_group: 'V-group',
  },
  impact: [
    { name: 'A-group', members: ['A', 'a'], total: 2 },
    { name: 'V-group', members: ['V', 'v'], total: 2 },
  ],
} as TextKerningModel
const glyphs = [
  { name: 'A', x: 10, y: 20 },
  { name: 'V', x: 570, y: 20 },
] as PositionedGlyph[]
const options = {
  doc: 1,
  master: 'regular',
  generation: 7,
  model,
  glyphs,
  selectedGap: 0,
  pending: false,
  direction: 'ltr',
}

afterEach(() => setTransport(undefined))

it('only loads when enabled and hides outlines immediately for stale shaping', async () => {
  const transport = createMockTransport({
    'glyphs.search': ({ query }) => [
      makeCell({ gid: query === 'a' ? 1 : 2, name: query }),
    ],
    'glyphs.cell': ({ gid }) =>
      makeCell({ gid, name: gid === 1 ? 'a' : 'v', path: 'M0 0L10 10Z' }),
  })
  setTransport(transport)
  const { result, rerender } = renderHook((props) => useKerningGhosts(props), {
    initialProps: options,
  })
  expect(result.current.value).toBe(-40)
  expect(transport.calls).toHaveLength(0)
  act(() => result.current.setEnabled(true))
  await waitFor(() => expect(result.current.outlines).toHaveLength(2))
  expect(result.current.outlines.map(({ x, y }) => [x, y])).toEqual([
    [10, 20],
    [570, 20],
  ])
  rerender({ ...options, pending: true })
  expect(result.current.outlines).toEqual([])
  expect(result.current.value).toBeNull()
  expect(result.current.loading).toBe(false)
})

it('discards old master responses and keeps the new master outlines', async () => {
  const releases: Array<(value: GlyphCell) => void> = []
  const transport = createMockTransport({
    'glyphs.search': ({ query }) => [
      makeCell({ gid: query === 'a' ? 1 : 2, name: query }),
    ],
    'glyphs.cell': ({ gid, master }) =>
      master === 'old'
        ? new Promise<GlyphCell>((resolve) => releases.push(resolve))
        : makeCell({ gid, name: gid === 1 ? 'a' : 'v', path: 'new' }),
  })
  setTransport(transport)
  const { result, rerender } = renderHook((props) => useKerningGhosts(props), {
    initialProps: { ...options, master: 'old' },
  })
  act(() => result.current.setEnabled(true))
  await waitFor(() => expect(releases).toHaveLength(2))
  rerender({ ...options, master: 'new' })
  await waitFor(() => expect(result.current.outlines).toHaveLength(2))
  await act(async () =>
    releases.forEach((release, index) =>
      release(
        makeCell({
          gid: index + 1,
          name: index === 0 ? 'a' : 'v',
          path: 'old',
        }),
      ),
    ),
  )
  expect(result.current.outlines.map(({ path }) => path)).toEqual([
    'new',
    'new',
  ])
})

it('hides the previous gap while a different pair resolution is arriving', () => {
  const { result } = renderHook(() =>
    useKerningGhosts({
      ...options,
      model: { ...model, pair: { left: 'B', right: 'V' } },
    }),
  )
  expect(result.current.available).toBe(false)
  expect(result.current.value).toBeNull()
})

it('offers a labeled native checkbox and signed feedback alongside color', async () => {
  const user = userEvent.setup()
  const setEnabled = vi.fn()
  render(
    <KerningGhostControls
      model={{
        enabled: false,
        available: true,
        loading: false,
        failed: false,
        outlines: [],
        value: -40,
        setEnabled,
      }}
    />,
  )
  const checkbox = screen.getByRole('checkbox', { name: 'Show group outlines' })
  expect(checkbox).toHaveAccessibleDescription(/four alternate members/)
  checkbox.focus()
  expect(checkbox).toHaveFocus()
  await user.keyboard('[Space]')
  expect(setEnabled).toHaveBeenCalledWith(true)
  expect(screen.getByText('Negative kerning: -40 units')).toHaveAttribute(
    'data-sign',
    'negative',
  )
})

it('aligns vertical ghost outlines using member origins and selected pen positions', async () => {
  const transport = createMockTransport({
    'glyphs.search': ({ query }) => [
      makeCell({ gid: query === 'a' ? 1 : 2, name: query }),
    ],
    'glyphs.cell': ({ gid }) =>
      makeCell({
        gid,
        name: gid === 1 ? 'a' : 'v',
        width: 800,
        vertical_origin: 850,
        path: 'M0 0L10 10Z',
      }),
  })
  setTransport(transport)
  const { result } = renderHook(() =>
    useKerningGhosts({
      ...options,
      direction: 'ttb',
      glyphs: glyphs.map((glyph, index) => ({
        ...glyph,
        penX: 50,
        penY: -index * 1000,
      })),
    }),
  )
  act(() => result.current.setEnabled(true))
  await waitFor(() => expect(result.current.outlines).toHaveLength(2))
  expect(result.current.outlines.map(({ x, y }) => [x, y])).toEqual([
    [-350, -850],
    [-350, -1850],
  ])
})

it('reports a failed outline request and retries when toggled off and on', async () => {
  let fail = true
  const transport = createMockTransport({
    'glyphs.search': ({ query }) => {
      if (fail) throw new Error('unavailable')
      return [makeCell({ gid: query === 'a' ? 1 : 2, name: query })]
    },
    'glyphs.cell': ({ gid }) =>
      makeCell({ gid, name: gid === 1 ? 'a' : 'v', path: 'M0 0Z' }),
  })
  setTransport(transport)
  const { result } = renderHook(() => useKerningGhosts(options))
  act(() => result.current.setEnabled(true))
  await waitFor(() => expect(result.current.failed).toBe(true))
  expect(result.current.outlines).toEqual([])
  fail = false
  act(() => result.current.setEnabled(false))
  act(() => result.current.setEnabled(true))
  await waitFor(() => expect(result.current.outlines).toHaveLength(2))
  expect(result.current.failed).toBe(false)
})
