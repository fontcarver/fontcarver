import { beforeEach, describe, expect, it, vi } from 'vitest'

import { initialUiState, type Tab, type TextTabState } from './uiStore'
import {
  captureWorkspace,
  initialWorkspaceState,
  restoreWorkspace,
  useWorkspaceStore,
  workspaceDocumentKey,
  WORKSPACE_STORAGE_KEY,
} from './workspaceStore'

const textState: TextTabState = {
  text: 'Hamburgefonts',
  activeIndex: 3,
  runCount: 12,
  shaping: true,
  mode: 'edit',
  master: 'regular',
  size: 72,
  features: [['kern', true]],
  direction: 'ltr',
  previewTheme: 'dark',
  previewCollapsed: false,
}

function snapshot() {
  const ui = initialUiState()
  const tabs: Tab[] = [
    { id: 'font:1', kind: 'font', doc: 1 },
    { id: 'font:2', kind: 'font', doc: 2 },
    { id: 'glyph:1:9', kind: 'glyph', doc: 1, gid: 9, label: 'A' },
    { id: 'glyph:1:10', kind: 'glyph', doc: 1, gid: 10, textState },
  ]
  return captureWorkspace(
    1,
    { ...ui, tabs, activeTab: 'glyph:1:9', sections: { metrics: false } },
    new Map([[9, 'A']]),
  )
}

beforeEach(() => {
  localStorage.clear()
  useWorkspaceStore.setState(initialWorkspaceState())
})

describe('workspace snapshot adapters', () => {
  it('restores the selected glyph by name under new document and glyph handles', () => {
    const saved = snapshot()
    expect(saved.tabs).toHaveLength(3)
    expect(saved.tabs[1]).toMatchObject({ kind: 'glyph', glyphName: 'A' })
    const restored = restoreWorkspace(saved, 77, new Map([['A', 501]]))
    expect(restored.tabs).toHaveLength(3)
    expect(restored.tabs.every((tab) => tab.doc === 77)).toBe(true)
    expect(restored.tabs[1]).toMatchObject({ gid: 501, label: 'A' })
    expect(restored.activeTab).toBe(restored.tabs[1]?.id)
    expect(restored.sections).toEqual({ metrics: false })
    expect(restored.tabs[2]?.textState).toMatchObject({
      text: 'Hamburgefonts',
      activeIndex: null,
      mode: 'text',
      shaping: false,
    })
    expect(restored.tabs[2]?.textState?.runCount).toBeUndefined()
  })

  it('skips unavailable glyphs and falls back to a valid tab', () => {
    const restored = restoreWorkspace(snapshot(), 77)
    expect(restored.tabs).toHaveLength(2)
    expect(restored.activeTab).toBe(restored.tabs[0]?.id)
    expect(restored.tabs[1]?.gid).toBeUndefined()
  })

  it('uses path keys and keeps unsaved documents distinct', () => {
    expect(workspaceDocumentKey({ doc: 1, path: '/fonts/one.fcf' })).toBe(
      workspaceDocumentKey({ doc: 9, path: '/fonts/one.fcf' }),
    )
    expect(workspaceDocumentKey({ doc: 1, path: null })).not.toBe(
      workspaceDocumentKey({ doc: 9, path: null }),
    )
  })
})

describe('saved workspaces', () => {
  it('keeps another window’s latest document and presets when saving stale local state', () => {
    const firstWindow = initialWorkspaceState()
    const saved = snapshot()
    useWorkspaceStore.getState().saveCurrent('path:/fonts/one.fcf', saved)
    useWorkspaceStore
      .getState()
      .savePreset('path:/fonts/one.fcf', 'Drawing', saved)
    useWorkspaceStore.setState(firstWindow)
    useWorkspaceStore.getState().saveCurrent('path:/fonts/two.fcf', {
      ...saved,
      textPreviewVisible: false,
    })
    const reloaded = initialWorkspaceState().documents
    expect(reloaded['path:/fonts/one.fcf']?.current).toEqual(saved)
    expect(reloaded['path:/fonts/one.fcf']?.presets[0]?.name).toBe('Drawing')
    expect(reloaded['path:/fonts/two.fcf']?.current?.textPreviewVisible).toBe(
      false,
    )
  })

  it('preserves a preset saved by another window of the same document', () => {
    const firstWindow = initialWorkspaceState()
    useWorkspaceStore
      .getState()
      .savePreset('path:/fonts/one.fcf', 'Drawing', snapshot())
    useWorkspaceStore.setState(firstWindow)
    useWorkspaceStore.getState().saveCurrent('path:/fonts/one.fcf', snapshot())
    expect(
      initialWorkspaceState().documents['path:/fonts/one.fcf']?.presets[0]
        ?.name,
    ).toBe('Drawing')
  })

  it('round trips current layout and named presets independently per document', () => {
    const state = useWorkspaceStore.getState()
    const saved = snapshot()
    state.saveCurrent('path:/fonts/one.fcf', saved)
    expect(state.savePreset('path:/fonts/one.fcf', ' Drawing ', saved)).toBe(
      true,
    )
    state.saveCurrent('path:/fonts/two.fcf', {
      ...saved,
      textPreviewVisible: false,
    })
    useWorkspaceStore.setState(initialWorkspaceState())
    const reloaded = useWorkspaceStore.getState()
    expect(reloaded.getCurrent('path:/fonts/one.fcf')).toEqual(saved)
    expect(reloaded.getCurrent('path:/fonts/two.fcf')?.textPreviewVisible).toBe(
      false,
    )
    expect(reloaded.applyPreset('path:/fonts/one.fcf', 'Drawing')).toEqual(
      saved,
    )
    expect(reloaded.applyPreset('path:/fonts/two.fcf', 'Drawing')).toBeNull()
    reloaded.deletePreset('path:/fonts/one.fcf', 'Drawing')
    useWorkspaceStore.setState(initialWorkspaceState())
    expect(
      useWorkspaceStore.getState().documents['path:/fonts/one.fcf']?.presets,
    ).toEqual([])
  })

  it('overwrites a named preset, rejects empty names and isolates returned snapshots', () => {
    const state = useWorkspaceStore.getState()
    state.savePreset('doc:1', 'Drawing', snapshot())
    state.savePreset('doc:1', 'Drawing', {
      ...snapshot(),
      textPreviewVisible: false,
    })
    expect(state.savePreset('doc:1', ' ', snapshot())).toBe(false)
    const restored = state.applyPreset('doc:1', 'Drawing')
    expect(restored?.textPreviewVisible).toBe(false)
    if (restored) restored.sizes.sidebar = 399
    expect(state.getCurrent('doc:1')?.sizes.sidebar).toBe(240)
    expect(
      useWorkspaceStore.getState().documents['doc:1']?.presets,
    ).toHaveLength(1)
  })

  it('migrates current and presets when an untitled document is saved', () => {
    const state = useWorkspaceStore.getState()
    state.saveCurrent('doc:1', snapshot())
    state.savePreset('doc:1', 'Drawing', snapshot())
    state.migrateDocumentKey('doc:1', 'path:/fonts/one.fcf')
    expect(state.getCurrent('doc:1')).toBeNull()
    expect(state.getCurrent('path:/fonts/one.fcf')).toEqual(snapshot())
    expect(state.applyPreset('path:/fonts/one.fcf', 'Drawing')).not.toBeNull()
  })

  it('ignores corrupt JSON, unsupported versions and malformed snapshots', () => {
    for (const raw of [
      '{',
      'null',
      JSON.stringify({ version: 999, documents: {} }),
      JSON.stringify({
        version: 1,
        documents: { invalid: { current: { tabs: 'oops' }, presets: [] } },
      }),
    ]) {
      localStorage.setItem(WORKSPACE_STORAGE_KEY, raw)
      expect(initialWorkspaceState().documents).toEqual({})
    }
  })

  it('clamps restored geometry and rejects malformed nested text state', () => {
    const saved = snapshot()
    localStorage.setItem(
      WORKSPACE_STORAGE_KEY,
      JSON.stringify({
        version: 1,
        documents: {
          valid: {
            current: {
              ...saved,
              sizes: { sidebar: -100, inspector: 99999 },
              tabs: [
                { kind: 'font' },
                { kind: 'glyph', textState: { text: [] } },
              ],
              activeTab: 500,
            },
            presets: [],
          },
        },
      }),
    )
    const loaded = initialWorkspaceState().documents.valid?.current
    expect(loaded?.sizes).toEqual({ sidebar: 200, inspector: 420 })
    expect(loaded?.tabs).toEqual([{ kind: 'font' }])
    expect(loaded?.activeTab).toBe(0)
  })

  it('keeps working when local storage cannot be read or written', () => {
    const read = vi
      .spyOn(Storage.prototype, 'getItem')
      .mockImplementation(() => {
        throw new Error('blocked')
      })
    expect(initialWorkspaceState().documents).toEqual({})
    read.mockRestore()
    const write = vi
      .spyOn(Storage.prototype, 'setItem')
      .mockImplementation(() => {
        throw new Error('quota')
      })
    useWorkspaceStore.getState().saveCurrent('doc:1', snapshot())
    expect(useWorkspaceStore.getState().getCurrent('doc:1')).not.toBeNull()
    write.mockRestore()
  })
})
