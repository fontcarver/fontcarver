import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { i18n } from '../i18n'
import * as persistence from '../editor/documentPersistence'
import {
  emptyFilter,
  initialDocumentState,
  useDocumentStore,
} from '../stores/documentStore'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { initialUiState, tabId, useUiStore } from '../stores/uiStore'
import { makeSummary } from '../test/mockTransport'

import {
  createPaletteFilters,
  openPaletteFilter,
  searchPaletteEntries,
  type PaletteEntry,
} from './paletteSearch'

beforeEach(() => {
  localStorage.clear()
  useUiStore.setState(initialUiState())
  useDocumentStore.setState(initialDocumentState())
  useSessionStore.setState({ ...initialSessionState(), docs: [makeSummary()] })
})
afterEach(() => vi.restoreAllMocks())

const entry = (id: string, kind: PaletteEntry['kind']): PaletteEntry => ({
  id,
  kind,
  label: id,
  enabled: true,
  run: vi.fn(),
})

describe('palette search routes', () => {
  it('filters the selected category before fuzzy ranking labels', () => {
    const entries = [
      entry('Open Font', 'commands'),
      entry('Open counters.js', 'scripts'),
      entry('Letter', 'filters'),
    ]
    expect(
      searchPaletteEntries('op', 'all', entries).map((item) => item.id),
    ).toEqual(['Open Font', 'Open counters.js'])
    expect(
      searchPaletteEntries('op', 'scripts', entries).map((item) => item.id),
    ).toEqual(['Open counters.js'])
  })

  it('exposes named taxonomy filters as actions', async () => {
    useUiStore.getState().openTab({ kind: 'glyph', doc: 1, gid: 7 })
    const filters = createPaletteFilters(
      1,
      {
        categories: [['Letter', 8]],
        scripts: [['Latn', 8]],
        blocks: [['Basic Latin', 8]],
        unencoded: 2,
        total: 10,
        color_labels: [[0, 2]],
      },
      i18n.t.bind(i18n),
    )
    await filters.find((item) => item.label === 'Basic Latin')?.run()
    expect(useDocumentStore.getState().byDoc[1]?.filter).toEqual({
      ...emptyFilter(),
      blocks: ['Basic Latin'],
    })
    expect(useUiStore.getState().activeTab).toBe(tabId('font', 1))
  })

  it('settles the active edits before replacing the filter and switching tabs', async () => {
    const glyph = useUiStore
      .getState()
      .openTab({ kind: 'glyph', doc: 1, gid: 7 })
    useDocumentStore
      .getState()
      .setFilter(1, { query: 'old', unencoded_only: true })
    let finish: () => void = () => undefined
    const pending = new Promise<void>((resolve) => {
      finish = resolve
    })
    vi.spyOn(persistence, 'settleDocumentEdits').mockReturnValue(pending)
    const opening = openPaletteFilter(1, {
      ...emptyFilter(),
      categories: ['Letter'],
    })
    expect(useUiStore.getState().activeTab).toBe(glyph)
    expect(useDocumentStore.getState().byDoc[1]?.filter.query).toBe('old')
    finish()
    await opening
    expect(useUiStore.getState().activeTab).toBe(tabId('font', 1))
    expect(useDocumentStore.getState().byDoc[1]?.filter).toEqual({
      ...emptyFilter(),
      categories: ['Letter'],
    })
  })

  it('does not apply a filter when the user navigated elsewhere during settlement', async () => {
    useUiStore.getState().openTab({ kind: 'glyph', doc: 1, gid: 7 })
    let finish: () => void = () => undefined
    vi.spyOn(persistence, 'settleDocumentEdits').mockReturnValue(
      new Promise<void>((resolve) => {
        finish = resolve
      }),
    )
    const opening = openPaletteFilter(1, {
      ...emptyFilter(),
      changed_only: true,
    })
    const other = useUiStore.getState().openTab({ kind: 'font', doc: 2 })
    finish()
    await opening
    expect(useUiStore.getState().activeTab).toBe(other)
    expect(useDocumentStore.getState().byDoc[1]).toBeUndefined()
  })
})
