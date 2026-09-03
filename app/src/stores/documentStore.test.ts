import { beforeEach, describe, expect, it, vi } from 'vitest'

import { setTransport, type SidebarCounts } from '../api'
import { createMockTransport } from '../test/mockTransport'

import { initialUiState, useUiStore } from './uiStore'

import {
  CELL_SIZE_MAX,
  CELL_SIZE_MIN,
  initialDocumentState,
  onDocumentForgotten,
  selectDocView,
  useDocumentStore,
} from './documentStore'

beforeEach(() => {
  localStorage.clear()
  useDocumentStore.setState(initialDocumentState())
  useUiStore.setState(initialUiState())
  setTransport(undefined)
})

describe('documentStore', () => {
  it('returns a stable empty view for unknown documents', () => {
    const select = selectDocView(99)
    const first = select(useDocumentStore.getState())
    const second = select(useDocumentStore.getState())

    expect(first).toBe(second)
    expect(first.cellSize).toBe(64)
  })

  it('clamps the cell size and steps through the preset sizes', () => {
    const store = useDocumentStore.getState()
    store.setCellSize(1, 5000)
    expect(useDocumentStore.getState().byDoc[1]?.cellSize).toBe(CELL_SIZE_MAX)

    store.setCellSize(1, 0)
    expect(useDocumentStore.getState().byDoc[1]?.cellSize).toBe(CELL_SIZE_MIN)

    store.stepCellSize(1, 1)
    expect(useDocumentStore.getState().byDoc[1]?.cellSize).toBe(32)

    store.stepCellSize(1, -1)
    expect(useDocumentStore.getState().byDoc[1]?.cellSize).toBe(24)
  })

  it('merges filter patches and keeps the rest of the filter', () => {
    useDocumentStore.getState().setFilter(1, { query: 'uni0041' })
    useDocumentStore.getState().setFilter(1, { categories: ['Letter'] })

    const filter = useDocumentStore.getState().byDoc[1]?.filter
    expect(filter?.query).toBe('uni0041')
    expect(filter?.categories).toEqual(['Letter'])
    expect(filter?.unencoded_only).toBe(false)
  })

  it('toggles and replaces the selection immutably', () => {
    const store = useDocumentStore.getState()
    store.toggleSelection(1, 5)
    store.toggleSelection(1, 6)
    expect([
      ...(useDocumentStore.getState().byDoc[1]?.selection ?? []),
    ]).toEqual([5, 6])

    store.toggleSelection(1, 5)
    expect([
      ...(useDocumentStore.getState().byDoc[1]?.selection ?? []),
    ]).toEqual([6])

    store.setSelection(1, [1, 2, 3])
    expect(useDocumentStore.getState().byDoc[1]?.selection.size).toBe(3)
    store.clearSelection(1)
    expect(useDocumentStore.getState().byDoc[1]?.selection.size).toBe(0)
  })

  it('loads sidebar counts through the transport', async () => {
    const counts = {
      categories: [['Letter', 100] as [string, number]],
      scripts: [],
      blocks: [],
      unencoded: 3,
      total: 120,
      color_labels: [],
    }
    setTransport(createMockTransport({ 'doc.sidebar': () => counts }))

    await useDocumentStore.getState().loadSidebar(7)

    expect(useDocumentStore.getState().byDoc[7]?.sidebar).toEqual(counts)
  })

  it('forgets a document view', () => {
    useDocumentStore.getState().setSort(1, 'Name')
    useDocumentStore.getState().forget(1)

    expect(useDocumentStore.getState().byDoc[1]).toBeUndefined()
  })

  it('drops a sidebar response that arrives after the document was closed', async () => {
    let resolveSidebar: (counts: SidebarCounts) => void = () => undefined
    setTransport(
      createMockTransport({
        'doc.sidebar': () =>
          new Promise<SidebarCounts>((resolve) => {
            resolveSidebar = resolve
          }),
      }),
    )

    const pending = useDocumentStore.getState().loadSidebar(3)
    expect(useDocumentStore.getState().byDoc[3]).toBeDefined()

    useDocumentStore.getState().forget(3)
    resolveSidebar({
      categories: [],
      scripts: [],
      blocks: [],
      unencoded: 0,
      total: 1,
      color_labels: [],
    })
    await pending

    expect(useDocumentStore.getState().byDoc[3]).toBeUndefined()
  })

  it('stays quiet when a request fails because the document was closed', async () => {
    let rejectSidebar: (error: Error) => void = () => undefined
    setTransport(
      createMockTransport({
        'doc.sidebar': () =>
          new Promise<SidebarCounts>((_resolve, reject) => {
            rejectSidebar = reject
          }),
      }),
    )

    const pending = useDocumentStore.getState().loadSidebar(4)
    useDocumentStore.getState().forget(4)
    rejectSidebar(new Error('unknown document'))
    await pending

    expect(useUiStore.getState().toasts).toEqual([])
    expect(useDocumentStore.getState().byDoc[4]).toBeUndefined()
  })
})

describe('document teardown', () => {
  it('tells per-document caches to release a forgotten document', () => {
    const released = vi.fn()
    const stop = onDocumentForgotten(released)
    useDocumentStore.getState().ensure(4)

    useDocumentStore.getState().forget(4)
    expect(released).toHaveBeenCalledWith(4)

    stop()
    useDocumentStore.getState().ensure(5)
    useDocumentStore.getState().forget(5)
    expect(released).toHaveBeenCalledTimes(1)
  })
})
