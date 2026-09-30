import { beforeEach, expect, it } from 'vitest'

import {
  initialDocumentState,
  useDocumentStore,
} from '../../stores/documentStore'

import {
  FONT_VIEW_PREFS_KEY,
  applyFontViewPrefs,
  forgetFontViewPrefs,
  readFontViewPrefs,
  useFontViewPrefs,
} from './fontViewPrefs'
import { DEFAULT_GLYPH_TABLE_COLUMNS } from './glyphTableColumns'

function stored() {
  return JSON.parse(localStorage.getItem(FONT_VIEW_PREFS_KEY) ?? 'null')
}

beforeEach(() => {
  localStorage.clear()
  forgetFontViewPrefs()
  useDocumentStore.setState(initialDocumentState())
  useFontViewPrefs.setState({
    viewMode: 'grid',
    cellSize: 64,
    labelMode: 'name',
    tableColumns: DEFAULT_GLYPH_TABLE_COLUMNS,
    referenceCharacters: true,
  })
})

it('persists the column order and set, keeping one column visible', () => {
  const prefs = useFontViewPrefs.getState()
  prefs.moveColumn('status', 'name')
  expect(useFontViewPrefs.getState().tableColumns[0]).toBe('status')
  expect(stored().tableColumns[0]).toBe('status')

  prefs.toggleColumn('width')
  expect(useFontViewPrefs.getState().tableColumns).not.toContain('width')
  // A hidden column returns in canonical order, not at the end.
  prefs.toggleColumn('subCategory')
  expect(useFontViewPrefs.getState().tableColumns).toEqual([
    'name',
    'unicode',
    'category',
    'subCategory',
    'script',
    'status',
  ])

  prefs.setTableColumns(['name'])
  prefs.toggleColumn('name')
  expect(useFontViewPrefs.getState().tableColumns).toEqual(['name'])
  prefs.setTableColumns([])
  expect(useFontViewPrefs.getState().tableColumns).toEqual(['name'])
})

it('reads back a stored presentation and repairs a corrupt one', () => {
  useFontViewPrefs.getState().remember({ viewMode: 'list', cellSize: 96 })
  useFontViewPrefs.getState().setReferenceCharacters(false)
  expect(readFontViewPrefs()).toMatchObject({
    viewMode: 'list',
    cellSize: 96,
    referenceCharacters: false,
  })

  localStorage.setItem(FONT_VIEW_PREFS_KEY, '{ not json')
  expect(readFontViewPrefs().viewMode).toBe('grid')
  localStorage.setItem(
    FONT_VIEW_PREFS_KEY,
    JSON.stringify({
      viewMode: 'mosaic',
      cellSize: 10_000,
      labelMode: 'nonsense',
      tableColumns: ['nope', 'status', 'status'],
      referenceCharacters: 'yes',
    }),
  )
  const repaired = readFontViewPrefs()
  expect(repaired.viewMode).toBe('grid')
  expect(repaired.cellSize).toBe(256)
  expect(repaired.labelMode).toBe('name')
  expect(repaired.tableColumns).toEqual(['status'])
})

it('applies the stored presentation to a document once and records changes', () => {
  useFontViewPrefs.getState().remember({
    viewMode: 'list',
    cellSize: 128,
    labelMode: 'unicode',
  })
  useDocumentStore.getState().ensure(7)
  applyFontViewPrefs(7)
  expect(useDocumentStore.getState().byDoc[7]).toMatchObject({
    viewMode: 'list',
    cellSize: 128,
    labelMode: 'unicode',
  })

  // A later change in the document store is remembered for the next document.
  useDocumentStore.getState().setCellSize(7, 48)
  expect(useFontViewPrefs.getState().cellSize).toBe(48)
  expect(stored().cellSize).toBe(48)

  useDocumentStore.getState().setViewMode(7, 'grid')
  useDocumentStore.getState().ensure(8)
  applyFontViewPrefs(8)
  expect(useDocumentStore.getState().byDoc[8]).toMatchObject({
    viewMode: 'grid',
    cellSize: 48,
  })
})

it('survives storage that cannot be read or written', () => {
  const getItem = Storage.prototype.getItem
  const setItem = Storage.prototype.setItem
  Storage.prototype.getItem = () => {
    throw new Error('blocked')
  }
  Storage.prototype.setItem = () => {
    throw new Error('blocked')
  }
  try {
    expect(readFontViewPrefs().cellSize).toBe(64)
    useFontViewPrefs.getState().setReferenceCharacters(false)
    expect(useFontViewPrefs.getState().referenceCharacters).toBe(false)
  } finally {
    Storage.prototype.getItem = getItem
    Storage.prototype.setItem = setItem
  }
})
