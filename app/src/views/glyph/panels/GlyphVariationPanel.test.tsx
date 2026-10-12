import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, expect, it } from 'vitest'
import { setTransport, type VariationPreview } from '../../../api'
import { initialEditorState, useEditorStore } from '../../../stores/editorStore'
import { useUiStore } from '../../../stores/uiStore'
import { useVariationPreviewStore } from '../../../stores/variationPreviewStore'
import {
  makeCell,
  makeGlyph,
  makeGlyphDetail,
  makeLayer,
  makeLayerOutline,
  makeMaster,
} from '../../../test/fontFixtures'
import { createMockTransport, noChange } from '../../../test/mockTransport'
import { buildScene } from '../glyphScene'
import { GlyphVariationPanel } from './GlyphVariationPanel'

afterEach(() => setTransport(undefined))
it('scrubs the current glyph beside the editor without changing the document', async () => {
  useUiStore.setState({ sections: { variation: false } })
  const transport = createMockTransport({
    'variation.state': () => ({
      generation: 3,
      axes: [
        {
          tag: [119, 103, 104, 116],
          name: 'Weight',
          min: 100,
          default: 400,
          max: 900,
          map: [],
          hidden: false,
        },
      ],
      masters: [makeMaster()],
      instances: [],
      first_glyph: 'anotherGlyph',
    }),
    'variation.preview': (params) => ({
      generation: 3,
      path: 'M0 0L100 0L100 200Z',
      width: params.location.wght ?? 400,
      vertical_width: null,
      vertical_origin: null,
      units_per_em: 1000,
      sources: [],
      shape_status: [],
      issues: [],
    }),
  })
  setTransport(transport)
  const view = render(<GlyphVariationPanel doc={1} glyph="uni6C34" />)
  await userEvent.click(
    screen.getByRole('button', { name: 'Interpolation preview' }),
  )
  const slider = await screen.findByRole('slider', { name: 'Scrub Weight' })
  // The scrubber steps in whole axis units, so a keyboard user reaches 500.
  expect(slider).toHaveAttribute('step', '1')
  expect(screen.getByRole('textbox', { name: 'Glyph name' })).toHaveAttribute(
    'readonly',
  )
  fireEvent.change(slider, { target: { value: '700' } })
  await waitFor(() =>
    expect(transport.calls).toContainEqual(
      expect.objectContaining({
        method: 'variation.preview',
        params: expect.objectContaining({
          glyph: 'uni6C34',
          location: { wght: 700 },
        }),
      }),
    ),
  )
  expect(
    transport.calls.some((call) => call.method === 'variation.apply'),
  ).toBe(false)
  await waitFor(() =>
    expect(useVariationPreviewStore.getState().preview).toMatchObject({
      doc: 1,
      glyph: 'uni6C34',
      width: 700,
    }),
  )
  await userEvent.click(
    screen.getByRole('checkbox', { name: 'Show on editing canvas' }),
  )
  expect(useVariationPreviewStore.getState().preview).toBeNull()
  await userEvent.click(
    screen.getByRole('checkbox', { name: 'Show on editing canvas' }),
  )
  expect(useVariationPreviewStore.getState().preview?.width).toBe(700)
  view.unmount()
  expect(useVariationPreviewStore.getState().preview).toBeNull()
})

it('refreshes edited sources and ignores a late preview from the previous glyph', async () => {
  useUiStore.setState({ sections: { variation: true } })
  let generation = 3
  const requests: {
    glyph: string
    resolve: (value: VariationPreview) => void
  }[] = []
  const transport = createMockTransport({
    'variation.state': () => ({
      generation,
      axes: [],
      masters: [makeMaster()],
      instances: [],
      first_glyph: 'A',
    }),
    'variation.preview': ({ glyph }) =>
      new Promise((resolve) => requests.push({ glyph, resolve })),
  })
  setTransport(transport)
  const preview = (width: number): VariationPreview => ({
    generation,
    path: `M0 0L${width} 0Z`,
    width,
    vertical_width: null,
    vertical_origin: null,
    units_per_em: 1000,
    sources: [],
    shape_status: [],
    issues: [],
  })
  const view = render(<GlyphVariationPanel doc={1} glyph="A" />)
  await waitFor(() => expect(requests).toHaveLength(1))
  view.rerender(<GlyphVariationPanel doc={1} glyph="B" />)
  await waitFor(() => expect(requests).toHaveLength(2))
  await act(async () => requests[1]!.resolve(preview(500)))
  expect(useVariationPreviewStore.getState().preview?.glyph).toBe('B')
  await act(async () => requests[0]!.resolve(preview(900)))
  expect(useVariationPreviewStore.getState().preview?.width).toBe(500)
  generation = 4
  act(() =>
    transport.emit({
      doc: 1,
      generation,
      change: noChange([7]),
      undo_label: null,
      redo_label: null,
      dirty: true,
    }),
  )
  await waitFor(() => expect(requests).toHaveLength(3))
  expect(useVariationPreviewStore.getState().preview).toBeNull()
  await act(async () => requests[2]!.resolve(preview(600)))
  expect(useVariationPreviewStore.getState().preview?.width).toBe(600)
  await userEvent.click(
    screen.getByRole('button', { name: 'Interpolation preview' }),
  )
  expect(useVariationPreviewStore.getState().preview).toBeNull()
})

it('clears the canvas when a refreshed interpolation request fails', async () => {
  useUiStore.setState({ sections: { variation: true } })
  let generation = 0
  const transport = createMockTransport({
    'variation.state': () => ({
      generation,
      axes: [],
      masters: [makeMaster()],
      instances: [],
      first_glyph: 'A',
    }),
    'variation.preview': () => {
      if (generation) throw new Error('preview unavailable')
      return {
        generation,
        path: 'M0 0L100 100Z',
        width: 500,
        vertical_width: null,
        vertical_origin: null,
        units_per_em: 1000,
        sources: [],
        shape_status: [],
        issues: [],
      }
    },
  })
  setTransport(transport)
  render(<GlyphVariationPanel doc={1} glyph="A" />)
  await waitFor(() =>
    expect(useVariationPreviewStore.getState().preview?.width).toBe(500),
  )
  generation++
  act(() =>
    transport.emit({
      doc: 1,
      generation,
      change: noChange([7]),
      undo_label: null,
      redo_label: null,
      dirty: true,
    }),
  )
  await screen.findByRole('alert')
  expect(useVariationPreviewStore.getState().preview).toBeNull()
})

it('edits an alternate layer into a compound condition set as one command', async () => {
  useUiStore.setState({ sections: { variation: true } })
  const alternate = makeLayer({
    id: 'bracket-1',
    name: '[500‹wght]',
    kind: {
      Bracket: {
        master: 'm1',
        axis: [119, 103, 104, 116],
        min: 500,
        max: null,
      },
    },
  })
  const glyph = makeGlyph({ name: 'A', layers: [makeLayer(), alternate] })
  useEditorStore.setState({
    ...initialEditorState(),
    activeGlyph: {
      doc: 1,
      gid: 3,
      glyph,
      scene: buildScene({
        detail: makeGlyphDetail(glyph),
        cell: makeCell({ gid: 3 }),
        master: makeMaster(),
        masterId: 'm1',
        unitsPerEm: 1000,
      }),
      cell: makeCell({ gid: 3 }),
      outline: makeLayerOutline(makeLayer()),
      master: 'm1',
      masterGuides: [],
      reload: () => undefined,
    },
  })
  const axis = (tag: number[], name: string) => ({
    tag: tag as [number, number, number, number],
    name,
    min: 100,
    default: 400,
    max: 900,
    map: [] as [number, number][],
    hidden: false,
  })
  const transport = createMockTransport({
    'variation.state': () => ({
      generation: 9,
      axes: [
        axis([119, 103, 104, 116], 'Weight'),
        axis([119, 100, 116, 104], 'Width'),
      ],
      masters: [makeMaster()],
      instances: [],
      first_glyph: 'A',
    }),
    'variation.preview': () => ({
      generation: 9,
      path: 'M0 0L100 0Z',
      width: 500,
      vertical_width: null,
      vertical_origin: null,
      units_per_em: 1000,
      sources: [],
      shape_status: [],
      issues: [],
    }),
    'variation.apply': () => noChange(),
  })
  setTransport(transport)
  render(<GlyphVariationPanel doc={1} glyph="A" />)
  expect(await screen.findByText('[500‹wght]')).toBeInTheDocument()
  const user = userEvent.setup()
  await user.click(screen.getByRole('button', { name: 'Edit condition' }))
  await user.click(screen.getByRole('button', { name: 'Add condition axis' }))
  await user.click(
    screen.getByRole('checkbox', { name: 'Inclusive maximum on wdth' }),
  )
  await user.click(screen.getByRole('button', { name: 'Apply changes' }))
  // One command for the whole region, so the gesture is one undo entry.
  await waitFor(() =>
    expect(
      transport.calls.find((call) => call.method === 'variation.apply')?.params,
    ).toMatchObject({
      expected_generation: 9,
      command: {
        Conditions: {
          glyph: 'A',
          from: [{ axis: [119, 103, 104, 116], min: 500 }],
          to: [
            { axis: [119, 103, 104, 116], min: 500 },
            { axis: [119, 100, 116, 104], min: 400, max: 900 },
          ],
          name: '[500‹wght, 400‹wdth‹900]',
        },
      },
    }),
  )
})

it('groups the sources of one alternate region into a single row', async () => {
  useUiStore.setState({ sections: { variation: true } })
  const alternate = (id: string, master: string) =>
    makeLayer({
      id,
      name: '[500‹wght]',
      kind: {
        Bracket: {
          master,
          axis: [119, 103, 104, 116],
          min: 500,
          max: null,
        },
      },
    })
  const glyph = makeGlyph({
    name: 'A',
    layers: [
      makeLayer(),
      alternate('bracket-1', 'm1'),
      alternate('bracket-2', 'm2'),
    ],
  })
  useEditorStore.setState({
    ...initialEditorState(),
    activeGlyph: {
      doc: 1,
      gid: 3,
      glyph,
      scene: buildScene({
        detail: makeGlyphDetail(glyph),
        cell: makeCell({ gid: 3 }),
        master: makeMaster(),
        masterId: 'm1',
        unitsPerEm: 1000,
      }),
      cell: makeCell({ gid: 3 }),
      outline: makeLayerOutline(makeLayer()),
      master: 'm1',
      masterGuides: [],
      reload: () => undefined,
    },
  })
  setTransport(
    createMockTransport({
      'variation.state': () => ({
        generation: 9,
        axes: [
          {
            tag: [119, 103, 104, 116] as [number, number, number, number],
            name: 'Weight',
            min: 100,
            default: 400,
            max: 900,
            map: [],
            hidden: false,
          },
        ],
        masters: [makeMaster()],
        instances: [],
        first_glyph: 'A',
      }),
      'variation.preview': () => ({
        generation: 9,
        path: '',
        width: 500,
        vertical_width: null,
        vertical_origin: null,
        units_per_em: 1000,
        sources: [],
        shape_status: [],
        issues: [],
      }),
    }),
  )
  render(<GlyphVariationPanel doc={1} glyph="A" />)
  expect(await screen.findByText('[500‹wght]')).toBeInTheDocument()
  expect(screen.getByText('2 sources')).toBeInTheDocument()
  expect(
    screen.getAllByRole('button', { name: 'Edit condition' }),
  ).toHaveLength(1)
})
