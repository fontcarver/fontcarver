import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { setTransport, type GlyphListResult } from '../api'
import { initialDocumentState, useDocumentStore } from '../stores/documentStore'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { initialUiState, tabId, useUiStore } from '../stores/uiStore'
import {
  captureWorkspace,
  useWorkspaceStore,
  workspaceDocumentKey,
} from '../stores/workspaceStore'
import { makeCell } from '../test/fontFixtures'
import { createMockTransport, makeSummary } from '../test/mockTransport'
import { forgetGlyphNames, rememberGlyphNames } from '../views/font/glyphNames'

import { installWorkspacePersistence } from './workspaceLifecycle'

let stop: (() => void) | undefined
const first = makeSummary({ path: '/fonts/first.fcf' })
const second = makeSummary({ doc: 2, path: '/fonts/second.fcf' })
const firstKey = workspaceDocumentKey(first)
const secondKey = workspaceDocumentKey(second)

function savedLayout(doc = first.doc) {
  return captureWorkspace(doc, {
    ...initialUiState(),
    tabs: [
      { id: tabId('font', doc), kind: 'font', doc },
      { id: tabId('features', doc), kind: 'features', doc },
    ],
    activeTab: tabId('features', doc),
    panels: { sidebar: false, inspector: true },
    sizes: { sidebar: 310, inspector: 390 },
    sections: { metrics: false },
    textPreviewVisible: false,
  })
}

beforeEach(() => {
  history.replaceState(null, '', '/')
  localStorage.clear()
  useUiStore.setState(initialUiState())
  useSessionStore.setState(initialSessionState())
  useDocumentStore.setState(initialDocumentState())
  useWorkspaceStore.setState({ documents: {} })
  forgetGlyphNames(1)
  forgetGlyphNames(2)
})

afterEach(() => {
  stop?.()
  stop = undefined
  setTransport(undefined)
  vi.restoreAllMocks()
  history.replaceState(null, '', '/')
})

describe('workspace lifecycle', () => {
  it.each(['tab&tab=font', 'panel&panel=sidebar'])(
    'leaves saved workspaces alone in a detached %s window',
    (route) => {
      history.replaceState(null, '', `/?doc=1&fcWindow=${route}`)
      const snapshot = savedLayout()
      useWorkspaceStore.getState().saveCurrent(firstKey, snapshot)
      useSessionStore.setState({ docs: [first] })
      stop = installWorkspacePersistence()

      useUiStore.getState().openTab({ kind: 'font', doc: 1 })
      useUiStore.getState().setPanelSize('sidebar', 270)

      expect(useUiStore.getState().tabs).toHaveLength(1)
      expect(useWorkspaceStore.getState().getCurrent(firstKey)).toEqual(
        snapshot,
      )
    },
  )

  it('reopens the windows a restored arrangement had detached', async () => {
    const layout = captureWorkspace(first.doc, {
      ...initialUiState(),
      tabs: [{ id: tabId('font', first.doc), kind: 'font', doc: first.doc }],
      activeTab: tabId('font', first.doc),
      detached: [
        { kind: 'panel', doc: first.doc, panel: 'inspector' },
        { kind: 'tab', doc: first.doc, tab: 'features', tabId: 'features:1' },
      ],
    })
    expect(layout.detached).toHaveLength(2)
    useWorkspaceStore.getState().saveCurrent(firstKey, layout)
    const opened: unknown[] = []
    const open = vi
      .spyOn(window, 'open')
      .mockImplementation((...args: unknown[]) => {
        opened.push(args[0])
        return { closed: false, focus: () => undefined } as unknown as Window
      })

    stop = installWorkspacePersistence()
    useSessionStore.setState({ docs: [first] })
    useUiStore.getState().openTab({ kind: 'font', doc: first.doc })

    await vi.waitFor(() => expect(opened).toHaveLength(2))
    expect(opened).toEqual([
      '?fcWindow=panel&doc=1&panel=inspector',
      '?fcWindow=tab&doc=1&tab=features&tabId=features%3A1',
    ])
    await vi.waitFor(() =>
      expect(useUiStore.getState().detached).toHaveLength(2),
    )
    open.mockRestore()
  })

  it('waits for a font tab, then restores tabs and the document layout', () => {
    useWorkspaceStore.getState().saveCurrent(firstKey, savedLayout())
    stop = installWorkspacePersistence()
    useSessionStore.setState({ docs: [first] })
    expect(useUiStore.getState().tabs).toEqual([])

    useUiStore.getState().openTab({ kind: 'font', doc: 1 })

    expect(useUiStore.getState()).toMatchObject({
      activeTab: tabId('features', 1),
      panels: { sidebar: false, inspector: true },
      sizes: { sidebar: 310, inspector: 390 },
      sections: { metrics: false },
      textPreviewVisible: false,
    })
    expect(useUiStore.getState().tabs.map((tab) => tab.kind)).toEqual([
      'font',
      'features',
    ])
  })

  it('keeps the first document’s restored active tab while starting a loaded session', async () => {
    useWorkspaceStore.getState().saveCurrent(firstKey, savedLayout())
    useWorkspaceStore.getState().saveCurrent(secondKey, {
      ...savedLayout(2),
      sizes: { sidebar: 380, inspector: 400 },
    })
    setTransport(
      createMockTransport({
        'session.list': () => [first, second],
        'session.recent': () => [],
      }),
    )
    const stopWorkspace = installWorkspacePersistence()
    const stopSession = useSessionStore.getState().start()
    stop = () => {
      stopSession()
      stopWorkspace()
    }

    await vi.waitFor(() => expect(useUiStore.getState().tabs).toHaveLength(4))

    expect(useUiStore.getState().activeTab).toBe(tabId('features', 1))
    expect(useUiStore.getState().sizes.sidebar).toBe(310)
    expect(
      useUiStore.getState().tabs.some((tab) => tab.id === tabId('features', 2)),
    ).toBe(true)
    expect(useWorkspaceStore.getState().getCurrent(firstKey)?.activeTab).toBe(1)
  })

  it('saves the previous document layout before restoring the incoming one', () => {
    useWorkspaceStore.getState().saveCurrent(secondKey, savedLayout(2))
    useSessionStore.setState({ docs: [first, second] })
    stop = installWorkspacePersistence()
    useUiStore.getState().openTab({ kind: 'font', doc: 1 })
    useUiStore.getState().setPanelSize('sidebar', 280)
    useUiStore.getState().openTab({ kind: 'font', doc: 2 })

    expect(
      useWorkspaceStore.getState().getCurrent(firstKey)?.sizes.sidebar,
    ).toBe(280)
    expect(useUiStore.getState().sizes.sidebar).toBe(310)
    useUiStore.getState().setActiveTab(tabId('font', 1))
    expect(useUiStore.getState().sizes.sidebar).toBe(280)
    expect(
      useUiStore.getState().tabs.some((tab) => tab.id === tabId('features', 2)),
    ).toBe(true)
  })

  it('restores background tabs without changing active document or panels', () => {
    useWorkspaceStore.getState().saveCurrent(secondKey, savedLayout(2))
    useSessionStore.setState({ docs: [first, second] })
    useUiStore.getState().openTab({ kind: 'font', doc: 1 })
    stop = installWorkspacePersistence()

    useUiStore.getState().openTab({ kind: 'font', doc: 2 }, false)

    expect(useUiStore.getState().activeTab).toBe(tabId('font', 1))
    expect(useUiStore.getState().panels.sidebar).toBe(true)
    expect(
      useUiStore.getState().tabs.some((tab) => tab.id === tabId('features', 2)),
    ).toBe(true)
  })

  it('migrates the current workspace when Save As changes a document path', () => {
    useSessionStore.setState({ docs: [first] })
    useUiStore.getState().openTab({ kind: 'font', doc: 1 })
    stop = installWorkspacePersistence()
    useUiStore.getState().setPanelSize('inspector', 370)
    const saved = { ...first, path: '/fonts/renamed.fcf' }

    useSessionStore.setState({ docs: [saved] })

    expect(
      useWorkspaceStore.getState().getCurrent(workspaceDocumentKey(saved))
        ?.sizes.inspector,
    ).toBe(370)
    expect(useWorkspaceStore.getState().getCurrent(firstKey)).toBeNull()
  })

  it('maps persisted glyph names to fresh session handles', async () => {
    const snapshot = savedLayout()
    snapshot.tabs.push({ kind: 'glyph', glyphName: 'A' })
    snapshot.activeTab = 2
    useWorkspaceStore.getState().saveCurrent(firstKey, snapshot)
    setTransport(
      createMockTransport({
        'glyphs.list': () => ({
          total: 1,
          items: [makeCell({ gid: 91, name: 'A' })],
        }),
      }),
    )
    useSessionStore.setState({ docs: [first] })
    stop = installWorkspacePersistence()
    useUiStore.getState().openTab({ kind: 'font', doc: 1 })

    await vi.waitFor(() => expect(useUiStore.getState().tabs).toHaveLength(3))
    expect(useUiStore.getState().tabs[2]?.gid).toBe(91)
    expect(useUiStore.getState().activeTab).toBe(
      useUiStore.getState().tabs[2]?.id,
    )
  })

  it('persists cached glyph names instead of session glyph handles', () => {
    useSessionStore.setState({ docs: [first] })
    rememberGlyphNames(1, [makeCell({ gid: 91, name: 'A' })])
    stop = installWorkspacePersistence()
    useUiStore.getState().openTab({ kind: 'font', doc: 1 })
    useUiStore.getState().openTab({ kind: 'glyph', doc: 1, gid: 91 })

    expect(
      useWorkspaceStore.getState().getCurrent(firstKey)?.tabs[1],
    ).toMatchObject({ kind: 'glyph', glyphName: 'A' })
  })

  it('records closing a background tab without adopting the active layout', () => {
    useWorkspaceStore.getState().saveCurrent(secondKey, savedLayout(2))
    useSessionStore.setState({ docs: [first, second] })
    useUiStore.getState().openTab({ kind: 'font', doc: 1 })
    stop = installWorkspacePersistence()
    useUiStore.getState().openTab({ kind: 'font', doc: 2 }, false)

    useUiStore.getState().closeTab(tabId('features', 2))

    const saved = useWorkspaceStore.getState().getCurrent(secondKey)
    expect(saved?.tabs.map((tab) => tab.kind)).toEqual(['font'])
    expect(saved?.panels.sidebar).toBe(false)
  })

  it.each(['edit', 'close', 'stop'] as const)(
    'discards a delayed restore after %s',
    async (action) => {
      let resolve!: (result: GlyphListResult) => void
      const pending = new Promise<GlyphListResult>((done) => {
        resolve = done
      })
      const snapshot = savedLayout()
      snapshot.tabs.push({ kind: 'glyph', glyphName: 'A' })
      useWorkspaceStore.getState().saveCurrent(firstKey, snapshot)
      setTransport(createMockTransport({ 'glyphs.list': () => pending }))
      useSessionStore.setState({ docs: [first] })
      stop = installWorkspacePersistence()
      useUiStore.getState().openTab({ kind: 'font', doc: 1 })
      if (action === 'edit') useUiStore.getState().setPanelSize('sidebar', 275)
      if (action === 'close') {
        useUiStore.getState().closeTabsForDoc(1)
        useSessionStore.setState({ docs: [] })
      }
      if (action === 'stop') stop()

      resolve({ total: 1, items: [makeCell({ gid: 91, name: 'A' })] })
      await pending
      await new Promise((done) => setTimeout(done, 0))

      expect(
        useUiStore.getState().tabs.some((tab) => tab.kind === 'features'),
      ).toBe(false)
      if (action === 'edit')
        expect(useUiStore.getState().sizes.sidebar).toBe(275)
      if (action === 'close') {
        expect(useUiStore.getState().tabs).toEqual([])
        expect(useWorkspaceStore.getState().getCurrent(firstKey)?.tabs).toEqual(
          snapshot.tabs,
        )
      }
    },
  )

  it('keeps a newly opened document on its Overview while restoring its tabs', async () => {
    // A document whose arrangement was remembered with a Features tab selected.
    const layout = captureWorkspace(second.doc, {
      ...initialUiState(),
      tabs: [
        { id: tabId('font', second.doc), kind: 'font', doc: second.doc },
        {
          id: tabId('features', second.doc),
          kind: 'features',
          doc: second.doc,
        },
      ],
      activeTab: tabId('features', second.doc),
    })
    useWorkspaceStore.getState().saveCurrent(secondKey, layout)

    stop = installWorkspacePersistence()
    useSessionStore.setState({ docs: [first] })
    useUiStore.getState().openTab({ kind: 'font', doc: first.doc })
    expect(useUiStore.getState().activeTab).toBe(tabId('font', first.doc))

    // Opening the second document activates its Overview, exactly as the
    // session store does; its remembered arrangement then takes over.
    useSessionStore.setState({ docs: [first, second] })
    useUiStore.getState().openTab({ kind: 'font', doc: second.doc })

    // The document the user just asked for is the one on screen.
    expect(useUiStore.getState().activeTab).toBe(tabId('font', second.doc))
    expect(
      useUiStore
        .getState()
        .tabs.filter((tab) => tab.doc === second.doc)
        .map((tab) => tab.kind),
    ).toEqual(['font', 'features'])
  })
})
