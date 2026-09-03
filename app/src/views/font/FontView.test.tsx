import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import type { GlyphCell as GlyphCellData, GlyphListParams } from '../../api'
import { setTransport } from '../../api'
import {
  initialDocumentState,
  useDocumentStore,
} from '../../stores/documentStore'
import { initialUiState, useUiStore } from '../../stores/uiStore'
import { createMockTransport, makeSummary } from '../../test/mockTransport'

import { FontView } from './FontView'

const NAMES = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H']

function cellFor(index: number): GlyphCellData {
  return {
    gid: index,
    name: NAMES[index % NAMES.length] ?? `g${index}`,
    unicodes: [0x41 + (index % NAMES.length)],
    category: 'Letter',
    sub_category: 'Uppercase',
    script: 'Latn',
    width: 1479,
    bbox: [0, 0, 1400, 1493],
    path: 'M0 0L100 0L100 100Z',
    color_label: null,
    export: true,
    has_components: false,
    is_empty: false,
    generation: 0,
  }
}

function transportFor(total: number) {
  return createMockTransport({
    'doc.sidebar': () => ({
      categories: [
        ['Letter', 5_000],
        ['Number', 200],
      ] as [string, number][],
      scripts: [['Latn', 4_000]] as [string, number][],
      blocks: [],
      unencoded: 12,
      total,
      color_labels: [[0, 3]] as [number, number][],
    }),
    'doc.font_info': () => ({
      info: {} as never,
      masters: [],
      tables: [],
      instances: [],
    }),
    'glyphs.list': (params: GlyphListParams) => ({
      total: params.filter.query === null ? total : 1,
      items: Array.from({ length: Math.min(params.limit, 8) }, (_, index) =>
        params.filter.query === null
          ? cellFor(params.offset + index)
          : cellFor(1),
      ).slice(0, params.filter.query === null ? 8 : 1),
    }),
  })
}

beforeEach(() => {
  localStorage.clear()
  useUiStore.setState(initialUiState())
  useDocumentStore.setState(initialDocumentState())
})

afterEach(() => {
  setTransport(undefined)
})

describe('FontView', () => {
  it('shows the match count and requests only the visible page', async () => {
    const transport = transportFor(6_253)
    setTransport(transport)

    render(<FontView summary={makeSummary()} />)

    expect(await screen.findByRole('option', { name: 'A' })).toBeInTheDocument()
    await waitFor(() =>
      expect(screen.getAllByText('6,253 glyphs').length).toBeGreaterThan(0),
    )

    const pages = transport.calls.filter(
      (entry) => entry.method === 'glyphs.list',
    )
    expect(pages).toHaveLength(1)
    expect(pages[0]?.params).toMatchObject({
      offset: 0,
      limit: 300,
      include_paths: true,
    })
  })

  it('sends the search text to the kernel after the debounce', async () => {
    const user = userEvent.setup()
    const transport = transportFor(6_253)
    setTransport(transport)

    render(<FontView summary={makeSummary()} />)
    await screen.findByRole('option', { name: 'A' })

    await user.type(
      screen.getByRole('searchbox', { name: 'Search glyphs' }),
      'U+0042',
    )

    await waitFor(() =>
      expect(
        transport.calls.some(
          (entry) =>
            entry.method === 'glyphs.list' &&
            (entry.params as GlyphListParams).filter.query === 'U+0042',
        ),
      ).toBe(true),
    )
    expect(await screen.findByRole('option', { name: 'B' })).toBeInTheDocument()
  })

  it('filters by a sidebar category and shows a removable chip', async () => {
    const user = userEvent.setup()
    setTransport(transportFor(6_253))
    const summary = makeSummary()

    render(
      <>
        <FontView summary={summary} />
      </>,
    )
    await screen.findByRole('option', { name: 'A' })
    useDocumentStore
      .getState()
      .setFilter(summary.doc, { categories: ['Letter'] })

    const chip = await screen.findByRole('button', {
      name: 'Remove filter Letter',
    })
    await user.click(chip)

    await waitFor(() =>
      expect(
        useDocumentStore.getState().byDoc[summary.doc]?.filter.categories,
      ).toEqual([]),
    )
  })

  // Pointer, keyboard, rubber-band and context-menu behaviour is covered in
  // GlyphGrid.test.tsx, which stubs a real viewport; this only checks that the
  // view wires a click through to the store.
  it('selects a glyph on click', async () => {
    const user = userEvent.setup()
    setTransport(transportFor(6_253))
    const summary = makeSummary()

    render(<FontView summary={summary} />)
    const first = await screen.findByRole('option', { name: 'A' })

    await user.click(first)
    await waitFor(() =>
      expect([
        ...(useDocumentStore.getState().byDoc[summary.doc]?.selection ?? []),
      ]).toEqual([0]),
    )
  })

  it('toggles the encoded-only filter from the toolbar', async () => {
    const user = userEvent.setup()
    setTransport(transportFor(6_253))
    const summary = makeSummary()

    render(<FontView summary={summary} />)
    await screen.findByRole('option', { name: 'A' })

    const toolbar = screen.getByRole('toolbar', { name: 'Glyph grid controls' })
    await user.click(
      within(toolbar).getByRole('button', { name: 'Encoded only' }),
    )

    await waitFor(() =>
      expect(
        useDocumentStore.getState().byDoc[summary.doc]?.filter.unicode_ranges,
      ).toEqual([[0, 0x10ffff]]),
    )
  })
})
