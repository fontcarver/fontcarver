import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { setTransport, type GlyphListParams } from '../../api'
import {
  initialDocumentState,
  useDocumentStore,
} from '../../stores/documentStore'
import { initialUiState, useUiStore } from '../../stores/uiStore'
import { glyphPage } from '../../test/glyphFixtures'
import { createMockTransport, makeSummary } from '../../test/mockTransport'

import { FontView } from './FontView'
import { forgetFontViewPrefs, readFontViewPrefs } from './fontViewPrefs'

const SUMMARY = makeSummary({ glyph_count: 65_535 })

function setup() {
  const mock = createMockTransport({
    'doc.font_info': () => ({
      info: {} as never,
      masters: [],
      tables: [],
      instances: [],
    }),
    'glyphs.list': (params: GlyphListParams) =>
      glyphPage(params.offset, params.limit, 65_535),
  })
  setTransport(mock)
  render(<FontView summary={SUMMARY} />)
  return mock
}

beforeEach(() => {
  localStorage.clear()
  forgetFontViewPrefs()
  vi.spyOn(HTMLElement.prototype, 'clientHeight', 'get').mockImplementation(
    function (this: HTMLElement) {
      return this.getAttribute('role') === 'rowgroup' ? 800 : 0
    },
  )
  useDocumentStore.setState(initialDocumentState())
  useUiStore.setState(initialUiState())
})

afterEach(() => {
  setTransport(undefined)
  vi.restoreAllMocks()
})

describe('glyph table', () => {
  it('switches views while preserving selection and renders a bounded range', async () => {
    setup()
    const glyph = await screen.findByRole('option', { name: 'B' })
    fireEvent.pointerDown(glyph, { button: 0 })
    fireEvent.click(screen.getByRole('button', { name: 'List view' }))
    const table = await screen.findByRole('grid', { name: 'Glyph list' })
    await waitFor(() =>
      expect(within(table).getByRole('row', { name: 'B' })).toHaveAttribute(
        'aria-selected',
        'true',
      ),
    )
    expect(within(table).getAllByRole('row').length).toBeLessThan(100)
    expect(within(table).getAllByRole('row').length).toBeGreaterThan(20)
    await waitFor(() =>
      expect(useDocumentStore.getState().byDoc[SUMMARY.doc]?.focus).toBe(1),
    )
    fireEvent.click(screen.getByRole('button', { name: 'Grid view' }))
    expect(await screen.findByRole('option', { name: 'B' })).toHaveAttribute(
      'aria-selected',
      'true',
    )
  })

  it('selects ranges, navigates rows and opens the focused glyph', async () => {
    useDocumentStore.getState().setViewMode(SUMMARY.doc, 'list')
    setup()
    const first = await screen.findByRole('row', { name: 'A' })
    fireEvent.pointerDown(first, { button: 0 })
    const rows = screen.getByRole('rowgroup', { name: 'Glyph rows' })
    fireEvent.keyDown(rows, { key: 'ArrowDown', shiftKey: true })
    await waitFor(() =>
      expect([
        ...useDocumentStore.getState().byDoc[SUMMARY.doc]!.selection,
      ]).toEqual([0, 1]),
    )
    fireEvent.keyDown(rows, { key: 'Enter' })
    await waitFor(() =>
      expect(useUiStore.getState().tabs.some((tab) => tab.gid === 1)).toBe(
        true,
      ),
    )
  })

  it('sorts through the kernel and retains reordered columns across view switches', async () => {
    useDocumentStore.getState().setViewMode(SUMMARY.doc, 'list')
    const mock = setup()
    await screen.findByRole('row', { name: 'A' })
    fireEvent.click(screen.getByRole('button', { name: 'Width' }))
    await waitFor(() =>
      expect(
        mock.calls.some(
          (call) =>
            call.method === 'glyphs.list' &&
            (call.params as GlyphListParams).sort === 'Width',
        ),
      ).toBe(true),
    )
    fireEvent.click(
      screen.getByRole('button', { name: 'Move Width column left' }),
    )
    const columnNames = () =>
      screen
        .getAllByRole('columnheader')
        .map((header) => header.getAttribute('data-column'))
    expect(columnNames().slice(0, 3)).toEqual(['name', 'width', 'unicode'])
    fireEvent.click(screen.getByRole('button', { name: 'Grid view' }))
    fireEvent.click(screen.getByRole('button', { name: 'List view' }))
    expect(columnNames().slice(0, 3)).toEqual(['name', 'width', 'unicode'])
  })

  it('chooses which columns appear and remembers the choice', async () => {
    useDocumentStore.getState().setViewMode(SUMMARY.doc, 'list')
    setup()
    await screen.findByRole('row', { name: 'A' })
    const columnNames = () =>
      screen
        .getAllByRole('columnheader')
        .map((header) => header.getAttribute('data-column'))
    expect(columnNames()).not.toContain('subCategory')

    fireEvent.click(screen.getByRole('button', { name: 'Choose columns' }))
    const menu = screen.getByRole('group', { name: 'Choose columns' })
    fireEvent.click(within(menu).getByRole('checkbox', { name: 'Subcategory' }))
    fireEvent.click(within(menu).getByRole('checkbox', { name: 'Width' }))
    expect(columnNames()).toEqual([
      'name',
      'unicode',
      'category',
      'subCategory',
      'script',
      'status',
    ])
    // The subcategory cell shows the kernel's classification for the row.
    const row = screen.getByRole('row', { name: 'A' })
    expect(within(row).getByTitle('Uppercase')).toBeInTheDocument()
    expect(readFontViewPrefs().tableColumns).toEqual(columnNames())
  })
})
