import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import type { DocumentEvent, GlyphListParams } from '../../api'
import { setTransport } from '../../api'
import { registerBuiltinCommands } from '../../shell/builtinCommands'
import { resetCommands } from '../../shell/commands'
import {
  initialDocumentState,
  useDocumentStore,
} from '../../stores/documentStore'
import { initialSessionState, useSessionStore } from '../../stores/sessionStore'
import { initialUiState, useUiStore } from '../../stores/uiStore'
import { glyphPage, stubGridViewport } from '../../test/glyphFixtures'
import { createMockTransport, makeSummary } from '../../test/mockTransport'

import { FontView } from './FontView'
import { resetScrollMemory } from './scrollMemory'

const TOTAL = 600
const SUMMARY = makeSummary({ glyph_count: TOTAL })
const DOC = SUMMARY.doc

let restoreViewport: () => void
let names: (index: number) => string

function transport() {
  return createMockTransport({
    'doc.sidebar': () => ({
      categories: [['Letter', 500]] as [string, number][],
      scripts: [],
      blocks: [],
      unencoded: 0,
      total: TOTAL,
      color_labels: [],
    }),
    'doc.font_info': () => ({
      info: {} as never,
      masters: [],
      tables: [],
      instances: [],
    }),
    'glyphs.list': (params: GlyphListParams) =>
      glyphPage(params.offset, params.limit, TOTAL, names),
  })
}

/** Renders the view and waits for the first page. */
async function mountGrid() {
  const mock = transport()
  setTransport(mock)
  useSessionStore.setState({ ...initialSessionState(), docs: [SUMMARY] })
  useUiStore.getState().openTab({ kind: 'font', doc: DOC })
  render(<FontView summary={SUMMARY} />)
  await screen.findByRole('option', { name: 'A' })
  return { mock, grid: screen.getByRole('listbox', { name: 'Glyphs' }) }
}

function selection(): number[] {
  return [...(useDocumentStore.getState().byDoc[DOC]?.selection ?? [])].sort(
    (left, right) => left - right,
  )
}

function cellAt(index: number): HTMLElement {
  const cell = document.querySelector(`[data-index="${index}"]`)
  if (!(cell instanceof HTMLElement)) throw new Error(`no cell at ${index}`)
  return cell
}

beforeEach(() => {
  localStorage.clear()
  resetScrollMemory()
  names = (index) =>
    index < 26 ? String.fromCharCode(65 + index) : `g${index}`
  restoreViewport = stubGridViewport()
  resetCommands()
  registerBuiltinCommands()
  useUiStore.setState(initialUiState())
  useSessionStore.setState(initialSessionState())
  useDocumentStore.setState(initialDocumentState())
})

afterEach(() => {
  restoreViewport()
  setTransport(undefined)
  resetCommands()
})

describe('glyph grid pointer selection', () => {
  it('replaces on click, extends on Shift and toggles on Ctrl', async () => {
    await mountGrid()

    fireEvent.pointerDown(cellAt(2), { button: 0 })
    await waitFor(() => expect(selection()).toEqual([2]))

    fireEvent.pointerDown(cellAt(5), { button: 0, shiftKey: true })
    await waitFor(() => expect(selection()).toEqual([2, 3, 4, 5]))

    fireEvent.pointerDown(cellAt(3), { button: 0, ctrlKey: true })
    await waitFor(() => expect(selection()).toEqual([2, 4, 5]))
  })

  it('extends a Shift range backwards from the anchor', async () => {
    await mountGrid()

    fireEvent.pointerDown(cellAt(20), { button: 0 })
    await waitFor(() => expect(selection()).toEqual([20]))
    fireEvent.pointerDown(cellAt(17), { button: 0, shiftKey: true })

    await waitFor(() => expect(selection()).toEqual([17, 18, 19, 20]))
  })

  it('selects a rectangle with a rubber band and clears on an empty click', async () => {
    const { grid } = await mountGrid()

    fireEvent.pointerDown(grid, { button: 0, clientX: 4, clientY: 4 })
    fireEvent.pointerMove(grid, { clientX: 230, clientY: 160 })
    fireEvent.pointerUp(grid, { clientX: 230, clientY: 160 })

    // 16 columns at a 72 px pitch: rows 0-2, columns 0-3.
    await waitFor(() =>
      expect(selection()).toEqual([0, 1, 2, 3, 16, 17, 18, 19, 32, 33, 34, 35]),
    )

    // 16 columns end at x = 1152, so this lands past the last one.
    fireEvent.pointerDown(grid, { button: 0, clientX: 1180, clientY: 700 })
    fireEvent.pointerUp(grid, { clientX: 1180, clientY: 700 })
    await waitFor(() => expect(selection()).toEqual([]))
  })
})

describe('glyph grid keyboard navigation', () => {
  it('moves with the arrow keys and wraps a row with Down', async () => {
    const { grid } = await mountGrid()
    fireEvent.pointerDown(cellAt(0), { button: 0 })
    await waitFor(() => expect(selection()).toEqual([0]))

    fireEvent.keyDown(grid, { key: 'ArrowRight' })
    await waitFor(() => expect(selection()).toEqual([1]))

    fireEvent.keyDown(grid, { key: 'ArrowDown' })
    await waitFor(() => expect(selection()).toEqual([17]))

    fireEvent.keyDown(grid, { key: 'ArrowUp' })
    await waitFor(() => expect(selection()).toEqual([1]))

    fireEvent.keyDown(grid, { key: 'ArrowLeft' })
    await waitFor(() => expect(selection()).toEqual([0]))
  })

  it('jumps to both ends with Home and End', async () => {
    const { grid } = await mountGrid()
    fireEvent.pointerDown(cellAt(3), { button: 0 })
    await waitFor(() => expect(selection()).toEqual([3]))

    fireEvent.keyDown(grid, { key: 'End' })
    await waitFor(() => expect(selection()).toEqual([TOTAL - 1]))

    fireEvent.keyDown(grid, { key: 'Home' })
    await waitFor(() => expect(selection()).toEqual([0]))
  })

  it('pages down and back up by whole screens', async () => {
    const { grid } = await mountGrid()
    fireEvent.pointerDown(cellAt(0), { button: 0 })
    await waitFor(() => expect(selection()).toEqual([0]))

    fireEvent.keyDown(grid, { key: 'PageDown' })
    let after: number[] = []
    await waitFor(() => {
      after = selection()
      expect(after).toHaveLength(1)
      expect(after[0]).toBeGreaterThan(16)
    })

    fireEvent.keyDown(grid, { key: 'PageUp' })
    await waitFor(() => expect(selection()).toEqual([0]))
  })

  it('extends the selection when an arrow key is pressed with Shift', async () => {
    const { grid } = await mountGrid()
    fireEvent.pointerDown(cellAt(4), { button: 0 })
    await waitFor(() => expect(selection()).toEqual([4]))

    fireEvent.keyDown(grid, { key: 'ArrowRight', shiftKey: true })
    await waitFor(() => expect(selection()).toEqual([4, 5]))

    fireEvent.keyDown(grid, { key: 'ArrowRight', shiftKey: true })
    await waitFor(() => expect(selection()).toEqual([4, 5, 6]))
  })

  it('clears the selection with Escape', async () => {
    const { grid } = await mountGrid()
    fireEvent.pointerDown(cellAt(9), { button: 0 })
    await waitFor(() => expect(selection()).toEqual([9]))

    fireEvent.keyDown(grid, { key: 'Escape' })
    await waitFor(() => expect(selection()).toEqual([]))
  })
})

describe('glyph grid opening', () => {
  it('opens the focused glyph with Enter', async () => {
    const { grid } = await mountGrid()
    fireEvent.pointerDown(cellAt(6), { button: 0 })
    await waitFor(() => expect(selection()).toEqual([6]))

    fireEvent.keyDown(grid, { key: 'Enter' })

    await waitFor(() =>
      expect(useUiStore.getState().tabs.some((tab) => tab.gid === 6)).toBe(
        true,
      ),
    )
  })

  it('opens the glyph under a double click', async () => {
    const { grid } = await mountGrid()
    fireEvent.pointerDown(cellAt(11), { button: 0 })
    await waitFor(() => expect(selection()).toEqual([11]))

    fireEvent.doubleClick(grid)

    await waitFor(() =>
      expect(useUiStore.getState().tabs.some((tab) => tab.gid === 11)).toBe(
        true,
      ),
    )
  })
})

describe('glyph grid context menu', () => {
  it('targets the clicked glyph without collapsing the selection', async () => {
    await mountGrid()
    fireEvent.pointerDown(cellAt(5), { button: 0 })
    await waitFor(() => expect(selection()).toEqual([5]))
    fireEvent.pointerDown(cellAt(8), { button: 0, shiftKey: true })
    await waitFor(() => expect(selection()).toEqual([5, 6, 7, 8]))

    fireEvent.contextMenu(cellAt(7), { clientX: 40, clientY: 40 })

    // The selection survives, but the clicked glyph becomes the target.
    expect(selection()).toEqual([5, 6, 7, 8])
    await waitFor(() =>
      expect(useDocumentStore.getState().byDoc[DOC]?.focus).toBe(7),
    )

    fireEvent.click(
      screen.getByRole('menuitem', { name: 'Open Glyph in New Tab' }),
    )
    await waitFor(() =>
      expect(useUiStore.getState().tabs.some((tab) => tab.gid === 7)).toBe(
        true,
      ),
    )
  })

  it('selects an unselected glyph before opening its menu', async () => {
    await mountGrid()
    fireEvent.pointerDown(cellAt(2), { button: 0 })
    await waitFor(() => expect(selection()).toEqual([2]))

    fireEvent.contextMenu(cellAt(9), { clientX: 40, clientY: 40 })

    await waitFor(() => expect(selection()).toEqual([9]))
  })

  it('focuses the first item and hands focus back to the grid on Escape', async () => {
    const { grid } = await mountGrid()
    fireEvent.pointerDown(cellAt(1), { button: 0 })
    await waitFor(() => expect(selection()).toEqual([1]))

    fireEvent.contextMenu(cellAt(1), { clientX: 40, clientY: 40 })
    const first = await screen.findByRole('menuitem', {
      name: 'Open Glyph in New Tab',
    })
    await waitFor(() => expect(document.activeElement).toBe(first))

    fireEvent.keyDown(document, { key: 'Escape' })
    await waitFor(() => expect(document.activeElement).toBe(grid))
  })
})

describe('glyph grid invalidation', () => {
  it('repaints a glyph the kernel reports as changed', async () => {
    const { mock } = await mountGrid()
    expect(await screen.findByRole('option', { name: 'C' })).toBeInTheDocument()

    names = (index) => (index === 2 ? 'C.alt' : `g${index}`)
    const event: DocumentEvent = {
      doc: DOC,
      generation: 4,
      change: {
        glyphs: [2],
        font_info: false,
        masters: false,
        layout: false,
        bitmaps: false,
        math: false,
        feature_source: false,
        glyph_order: false,
        metrics: false,
        layers: false,
        guides: false,
      },
      undo_label: 'Rename',
      redo_label: null,
      dirty: true,
    }
    mock.emit(event)

    expect(
      await screen.findByRole('option', { name: 'C.alt' }, { timeout: 2_000 }),
    ).toBeInTheDocument()
  })

  it('keeps the scroll position across a full reload', async () => {
    const { mock, grid } = await mountGrid()
    await screen.findByRole('option', { name: 'A' })

    // Ten rows down a 72 px pitch, the way a user leaves the top of the list.
    grid.scrollTop = 720
    fireEvent.scroll(grid)
    await waitFor(() => expect(cellAt(160)).toBeInTheDocument())

    names = (index) => `z${index}`
    mock.emit({
      doc: DOC,
      generation: 12,
      change: {
        glyphs: [],
        font_info: false,
        masters: false,
        layout: false,
        bitmaps: false,
        math: false,
        feature_source: false,
        glyph_order: true,
        metrics: false,
        layers: false,
        guides: false,
      },
      undo_label: 'Reorder glyphs',
      redo_label: null,
      dirty: true,
    })
    // A browser clamps the offset while the reloading list has no height.
    grid.scrollTop = 0
    fireEvent.scroll(grid)

    await waitFor(() => expect(grid.scrollTop).toBe(720))
    expect(await screen.findByRole('option', { name: 'z160' })).toBeVisible()
  })

  it('refreshes the em box when the kernel reports changed masters', async () => {
    const { mock } = await mountGrid()
    await screen.findByRole('option', { name: 'A' })
    const before = mock.calls.filter(
      (call) => call.method === 'doc.font_info',
    ).length

    mock.emit({
      doc: DOC,
      generation: 5,
      change: {
        glyphs: [],
        font_info: false,
        masters: true,
        layout: false,
        bitmaps: false,
        math: false,
        feature_source: false,
        glyph_order: false,
        metrics: false,
        layers: false,
        guides: false,
      },
      undo_label: 'Edit metrics',
      redo_label: null,
      dirty: true,
    })

    await waitFor(() =>
      expect(
        mock.calls.filter((call) => call.method === 'doc.font_info').length,
      ).toBeGreaterThan(before),
    )
  })

  it('reloads the whole list when the glyph order changed', async () => {
    const { mock } = await mountGrid()
    await screen.findByRole('option', { name: 'A' })

    names = (index) => `z${index}`
    mock.emit({
      doc: DOC,
      generation: 9,
      change: {
        glyphs: [],
        font_info: false,
        masters: false,
        layout: false,
        bitmaps: false,
        math: false,
        feature_source: false,
        glyph_order: true,
        metrics: false,
        layers: false,
        guides: false,
      },
      undo_label: null,
      redo_label: null,
      dirty: true,
    })

    expect(
      await screen.findByRole('option', { name: 'z0' }, { timeout: 2_000 }),
    ).toBeInTheDocument()
  })
})
