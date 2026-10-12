import { act, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import type { SidebarCounts } from '../../api'
import { setTransport } from '../../api'
import {
  initialDocumentState,
  useDocumentStore,
} from '../../stores/documentStore'
import { initialUiState, useUiStore } from '../../stores/uiStore'
import { createMockTransport } from '../../test/mockTransport'

import { SidebarFilters } from './SidebarFilters'

const DOC = 1

function counts(letters: number): SidebarCounts {
  return {
    categories: [['Letter', letters]],
    scripts: [['Latn', 40]],
    blocks: [['Basic Latin', 95]],
    unencoded: 3,
    total: letters + 100,
    color_labels: [[0, 2]],
  }
}

let letters = 5_000

function transport() {
  return createMockTransport({
    'doc.sidebar': () => counts(letters),
    'inventory.get_lists': () => ({ lists: [] }),
    'glyph.smart_filters.get': () => ({ filters: [] }),
  })
}

beforeEach(() => {
  letters = 5_000
  useDocumentStore.setState(initialDocumentState())
  useUiStore.setState(initialUiState())
  useDocumentStore.getState().ensure(DOC)
})

afterEach(() => {
  setTransport(undefined)
})

describe('SidebarFilters', () => {
  it('loads the taxonomy counts once for an unedited document', async () => {
    const mock = transport()
    setTransport(mock)

    render(<SidebarFilters doc={DOC} />)

    expect(await screen.findByText('5,000')).toBeInTheDocument()
    expect(
      mock.calls.filter((entry) => entry.method === 'doc.sidebar'),
    ).toHaveLength(1)
  })

  it('refreshes the counts after an edit, keeping the old ones visible', async () => {
    const mock = transport()
    setTransport(mock)
    render(<SidebarFilters doc={DOC} />)
    await screen.findByText('5,000')

    letters = 4_900
    act(() => useDocumentStore.getState().noteGeneration(DOC, 3))

    // The counts on screen do not blank out while the new ones are computed.
    expect(screen.getByText('5,000')).toBeInTheDocument()

    expect(
      await screen.findByText('4,900', {}, { timeout: 2_000 }),
    ).toBeInTheDocument()
    expect(
      mock.calls.filter((entry) => entry.method === 'doc.sidebar'),
    ).toHaveLength(2)
  })

  it('collapses a burst of edits into one refresh', async () => {
    const mock = transport()
    setTransport(mock)
    render(<SidebarFilters doc={DOC} />)
    await screen.findByText('5,000')

    letters = 4_800
    act(() => {
      const store = useDocumentStore.getState()
      store.noteGeneration(DOC, 2)
      store.noteGeneration(DOC, 3)
      store.noteGeneration(DOC, 4)
    })

    expect(
      await screen.findByText('4,800', {}, { timeout: 2_000 }),
    ).toBeInTheDocument()
    expect(
      mock.calls.filter((entry) => entry.method === 'doc.sidebar'),
    ).toHaveLength(2)
  })

  it('toggles a category filter and marks the row pressed', async () => {
    const user = userEvent.setup()
    setTransport(transport())
    render(<SidebarFilters doc={DOC} />)
    const row = await screen.findByRole('button', { name: /^Letter/ })

    await user.click(row)
    await waitFor(() =>
      expect(useDocumentStore.getState().byDoc[DOC]?.filter.categories).toEqual(
        ['Letter'],
      ),
    )
    expect(row).toHaveAttribute('aria-pressed', 'true')

    await user.click(row)
    await waitFor(() =>
      expect(useDocumentStore.getState().byDoc[DOC]?.filter.categories).toEqual(
        [],
      ),
    )
  })

  it('selects a qualified subcategory without retaining its broad parent', async () => {
    const user = userEvent.setup()
    setTransport(
      createMockTransport({
        'doc.sidebar': () => ({
          ...counts(10),
          sub_categories: [
            ['Letter', 'Uppercase', 4],
            ['Letter', 'Other', 6],
          ],
        }),
        'inventory.get_lists': () => ({ lists: [] }),
        'glyph.smart_filters.get': () => ({ filters: [] }),
      }),
    )
    render(<SidebarFilters doc={DOC} />)
    await user.click(await screen.findByRole('button', { name: /^Letter/ }))
    await user.click(screen.getByRole('button', { name: /^Uppercase/ }))
    expect(useDocumentStore.getState().byDoc[DOC].filter.categories).toEqual([
      'Letter/Uppercase',
    ])
    await user.click(screen.getByRole('button', { name: /^Other/ }))
    expect(useDocumentStore.getState().byDoc[DOC].filter.categories).toEqual([
      'Letter/Uppercase',
      'Letter/Other',
    ])
    await user.click(screen.getByRole('button', { name: /^Letter/ }))
    expect(useDocumentStore.getState().byDoc[DOC].filter.categories).toEqual([
      'Letter',
    ])
  })

  it('badges a section row with its repertoire coverage and offers Generate', async () => {
    const user = userEvent.setup()
    setTransport(
      createMockTransport({
        'doc.sidebar': () => counts(2),
        'inventory.get_lists': () => ({ lists: [] }),
        'glyph.smart_filters.get': () => ({ filters: [] }),
        'inventory.coverage': () => ({
          categories: [
            { value: 'Letter', present: 2, total: 1_000, glyphs: 2 },
          ],
          sub_categories: [],
          scripts: [{ value: 'Latn', present: 40, total: 1_481, glyphs: 40 }],
          blocks: [
            { value: 'Basic Latin', present: 95, total: 128, glyphs: 95 },
          ],
          total_glyphs: 102,
        }),
        'inventory.missing': () => ({
          scalars: [0x0021, 0x0022],
          total: 33,
          recipe: 'U+0021 U+0022',
        }),
      }),
    )
    render(<SidebarFilters doc={DOC} />)

    const block = await screen.findByRole('button', { name: /^Basic Latin/ })
    expect(block).toHaveTextContent('95 / 128')
    expect(block.title).toContain('95 glyphs')
    expect(block.title).toContain('33 missing')

    const generate = screen.getByRole('button', {
      name: 'Preview 33 missing characters for Basic Latin',
    })
    await user.click(generate)
    await waitFor(() =>
      expect(useUiStore.getState().dialog).toEqual({
        kind: 'addGlyphs',
        doc: DOC,
        recipe: 'U+0021 U+0022',
      }),
    )
    // Only part of the repertoire was preloaded, so the person is told.
    expect(useUiStore.getState().toasts.at(-1)?.message).toContain('2 of 33')
  })

  it('filters by a cell badge through the status panel', async () => {
    const user = userEvent.setup()
    setTransport(transport())
    render(<SidebarFilters doc={DOC} />)
    const row = await screen.findByRole('button', { name: 'Locked' })

    await user.click(row)
    expect(
      useDocumentStore.getState().byDoc[DOC]?.filter.smart_expression,
    ).toBe('locked:"true"')
    expect(row).toHaveAttribute('aria-pressed', 'true')

    await user.click(
      screen.getByRole('button', {
        name: 'Placeholder for a missing component base',
      }),
    )
    expect(
      useDocumentStore.getState().byDoc[DOC]?.filter.smart_expression,
    ).toBe('is_placeholder:"true"')

    await user.click(screen.getByRole('button', { name: 'Not exported' }))
    expect(
      useDocumentStore.getState().byDoc[DOC]?.filter.smart_expression,
    ).toBe('export:"false"')
    expect(row).toHaveAttribute('aria-pressed', 'false')

    await user.click(screen.getByRole('button', { name: 'Not exported' }))
    expect(
      useDocumentStore.getState().byDoc[DOC]?.filter.smart_expression,
    ).toBeNull()
  })
})
