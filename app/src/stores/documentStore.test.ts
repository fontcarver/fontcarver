import { beforeEach, describe, expect, it } from 'vitest'

import { setTransport } from '../api'
import { createMockTransport } from '../test/mockTransport'

import {
  CELL_SIZE_MAX,
  CELL_SIZE_MIN,
  initialDocumentState,
  selectDocView,
  useDocumentStore,
} from './documentStore'

beforeEach(() => {
  useDocumentStore.setState(initialDocumentState())
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
})
