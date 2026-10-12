import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { setTransport, type GlyphListView } from '../../api'
import {
  initialDocumentState,
  useDocumentStore,
} from '../../stores/documentStore'
import { initialUiState, useUiStore } from '../../stores/uiStore'
import { cellFor } from '../../test/glyphFixtures'
import { createMockTransport } from '../../test/mockTransport'

import { GlyphListsPanel } from './GlyphListsPanel'
import { forgetGlyphNames, rememberGlyphNames } from './glyphNames'

const DOC = 1
const INITIAL: GlyphListView = {
  id: 'latin',
  name: 'Latin basics',
  glyphs: ['A', 'B', 'C'],
  present: ['A'],
  missing: ['B', 'C'],
}

function transport() {
  let lists: GlyphListView[] = [INITIAL]
  return createMockTransport({
    'inventory.get_lists': () => ({ lists }),
    'inventory.set_lists': ({ lists: update }) => {
      lists = update.map((list) => ({
        ...list,
        present: list.glyphs.filter((name) => name === 'A'),
        missing: list.glyphs.filter((name) => name !== 'A'),
      }))
      return { lists }
    },
  })
}

beforeEach(() => {
  useDocumentStore.setState(initialDocumentState())
  useUiStore.setState(initialUiState())
  useDocumentStore.getState().ensure(DOC)
})
afterEach(() => {
  forgetGlyphNames(DOC)
  setTransport(undefined)
})

describe('GlyphListsPanel', () => {
  it('shows saved list coverage, exact membership, and previews only missing names', async () => {
    const user = userEvent.setup()
    const mock = transport()
    setTransport(mock)
    render(<GlyphListsPanel doc={DOC} />)
    const row = await screen.findByRole('button', {
      name: 'Latin basics 1 / 3',
    })
    await user.click(row)
    expect(useDocumentStore.getState().byDoc[DOC]?.filter.glyph_names).toEqual([
      'A',
      'B',
      'C',
    ])
    expect(row).toHaveAttribute('aria-pressed', 'true')
    await user.click(
      screen.getByRole('button', { name: 'Generate 2 missing glyphs' }),
    )
    expect(useUiStore.getState().dialog).toEqual({
      kind: 'addGlyphs',
      doc: DOC,
      recipe: 'B C',
    })
    expect(
      mock.calls.some((entry) => entry.method === 'inventory.set_lists'),
    ).toBe(false)
    act(() =>
      useDocumentStore.getState().setFilter(DOC, { categories: ['Letter'] }),
    )
    expect(row).toHaveAttribute('aria-pressed', 'false')
  })

  it('creates a named list from selection and supports imported names, rename and delete', async () => {
    const user = userEvent.setup()
    setTransport(transport())
    rememberGlyphNames(DOC, [cellFor(0), cellFor(1)])
    useDocumentStore.getState().setSelection(DOC, [0, 1])
    const rendered = render(<GlyphListsPanel doc={DOC} />)
    await screen.findByRole('button', { name: 'Latin basics 1 / 3' })
    await user.click(screen.getByRole('button', { name: 'New glyph list' }))
    fireEvent.change(screen.getByRole('textbox', { name: 'List name' }), {
      target: { value: 'Proof' },
    })
    await user.click(
      screen.getByRole('button', { name: 'Use 2 selected glyphs' }),
    )
    expect(screen.getByRole('textbox', { name: 'Glyph names' })).toHaveValue(
      'A\nB',
    )
    fireEvent.change(screen.getByRole('textbox', { name: 'Glyph names' }), {
      target: { value: 'A\nB\nB\nC' },
    })
    await user.click(screen.getByRole('button', { name: 'Save' }))
    await screen.findByRole('button', { name: 'Proof 1 / 3' })
    rendered.unmount()
    render(<GlyphListsPanel doc={DOC} />)
    await screen.findByRole('button', { name: 'Proof 1 / 3' })
    await user.click(screen.getByRole('button', { name: 'Edit Proof' }))
    fireEvent.change(screen.getByRole('textbox', { name: 'List name' }), {
      target: { value: 'Review' },
    })
    await user.click(screen.getByRole('button', { name: 'Save' }))
    await screen.findByRole('button', { name: 'Review 1 / 3' })
    await user.click(screen.getByRole('button', { name: 'Edit Review' }))
    await user.click(screen.getByRole('button', { name: 'Delete list' }))
    await waitFor(() =>
      expect(
        screen.queryByRole('button', { name: 'Review 1 / 3' }),
      ).not.toBeInTheDocument(),
    )
  })

  it('refreshes saved list contents on undo without losing an unsaved draft', async () => {
    const user = userEvent.setup()
    let lists = [INITIAL]
    setTransport(
      createMockTransport({ 'inventory.get_lists': () => ({ lists }) }),
    )
    render(<GlyphListsPanel doc={DOC} />)
    await screen.findByRole('button', { name: 'Latin basics 1 / 3' })
    await user.click(screen.getByRole('button', { name: 'New glyph list' }))
    fireEvent.change(screen.getByRole('textbox', { name: 'List name' }), {
      target: { value: 'Draft' },
    })
    lists = []
    act(() => useDocumentStore.getState().noteGeneration(DOC, 1))
    await waitFor(() =>
      expect(
        screen.queryByRole('button', { name: 'Latin basics 1 / 3' }),
      ).not.toBeInTheDocument(),
    )
    expect(screen.getByRole('textbox', { name: 'List name' })).toHaveValue(
      'Draft',
    )
  })

  it('ignores an old document response after switching documents', async () => {
    let resolveOld: ((value: { lists: GlyphListView[] }) => void) | undefined
    setTransport(
      createMockTransport({
        'inventory.get_lists': ({ doc }) =>
          doc === DOC
            ? new Promise((resolve) => {
                resolveOld = resolve
              })
            : { lists: [{ ...INITIAL, id: 'new', name: 'Other document' }] },
      }),
    )
    const rendered = render(<GlyphListsPanel doc={DOC} />)
    rendered.rerender(<GlyphListsPanel doc={2} />)
    await screen.findByRole('button', { name: 'Other document 1 / 3' })
    await act(async () => {
      resolveOld?.({ lists: [INITIAL] })
    })
    expect(
      screen.queryByRole('button', { name: 'Latin basics 1 / 3' }),
    ).not.toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: 'Other document 1 / 3' }),
    ).toBeInTheDocument()
  })

  it('keeps the draft and saved list when persistence fails', async () => {
    const user = userEvent.setup()
    setTransport(
      createMockTransport({
        'inventory.get_lists': () => ({ lists: [INITIAL] }),
        'inventory.set_lists': () => {
          throw new Error('Save failed')
        },
      }),
    )
    render(<GlyphListsPanel doc={DOC} />)
    await user.click(
      await screen.findByRole('button', { name: 'Edit Latin basics' }),
    )
    fireEvent.change(screen.getByRole('textbox', { name: 'List name' }), {
      target: { value: 'Unsaved' },
    })
    await user.click(screen.getByRole('button', { name: 'Save' }))
    await waitFor(() => expect(useUiStore.getState().toasts).toHaveLength(1))
    expect(screen.getByRole('textbox', { name: 'List name' })).toHaveValue(
      'Unsaved',
    )
    expect(
      screen.getByRole('button', { name: 'Latin basics 1 / 3' }),
    ).toBeInTheDocument()
  })
})
