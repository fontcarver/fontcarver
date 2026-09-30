import { call, type DocId, type GlyphId } from '../api'
import { i18n } from '../i18n'
import { emptyFilter, useDocumentStore } from '../stores/documentStore'
import { useSessionStore } from '../stores/sessionStore'
import { selectActiveDoc, useUiStore, type UiState } from '../stores/uiStore'
import {
  captureWorkspace,
  restoreWorkspace,
  useWorkspaceStore,
  workspaceDocumentKey,
} from '../stores/workspaceStore'
import { readWindowTarget } from '../transport/windows'
import { glyphName } from '../views/font/glyphNames'
import { reopenDetachedViews } from './windowActions'

type Snapshot = ReturnType<typeof captureWorkspace>

interface OpenWorkspace {
  key: string
  initialized: boolean
  pending: boolean
  captureVersion: number
  names?: Map<GlyphId, string>
  generation?: number
  request?: Promise<Map<GlyphId, string> | undefined>
}

function layoutChanged(a: UiState, b: UiState): boolean {
  return (
    a.panels !== b.panels ||
    a.sizes !== b.sizes ||
    a.sections !== b.sections ||
    a.textPreviewVisible !== b.textPreviewVisible
  )
}

function tabsChanged(doc: DocId, a: UiState, b: UiState): boolean {
  const before = a.tabs.filter((tab) => tab.doc === doc)
  const after = b.tabs.filter((tab) => tab.doc === doc)
  return (
    before.length !== after.length || before.some((tab, i) => tab !== after[i])
  )
}

/** Keeps each open document's workspace in sync with its persisted layout. */
export function installWorkspacePersistence(): () => void {
  const target = readWindowTarget()
  if (target && target.kind !== 'document') return () => {}
  const opened = new Map<DocId, OpenWorkspace>()
  let disposed = false
  let applying = false

  const live = (doc: DocId, entry: OpenWorkspace) =>
    !disposed && opened.get(doc) === entry
  const generation = (doc: DocId) =>
    useDocumentStore.getState().byDoc[doc]?.generation ?? 0

  const loadNames = (
    doc: DocId,
    entry: OpenWorkspace,
  ): Promise<Map<GlyphId, string> | undefined> => {
    if (entry.names && entry.generation === generation(doc))
      return Promise.resolve(entry.names)
    if (entry.request) return entry.request
    const startedAt = generation(doc)
    const request = (async () => {
      const names = new Map<GlyphId, string>()
      let offset = 0
      for (;;) {
        const page = await call('glyphs.list', {
          doc,
          master: useDocumentStore.getState().byDoc[doc]?.master ?? null,
          filter: emptyFilter(),
          sort: 'Order',
          offset,
          limit: 2_000,
          include_paths: false,
        })
        if (!live(doc, entry) || generation(doc) !== startedAt) return
        for (const cell of page.items) names.set(cell.gid, cell.name)
        offset += page.items.length
        if (offset >= page.total || page.items.length === 0) break
      }
      entry.names = names
      entry.generation = startedAt
      return names
    })()
      // Workspace restoration should not interrupt opening the document when
      // glyph metadata cannot be read. A later UI change can retry capture.
      .catch(() => undefined)
      .finally(() => {
        if (entry.request === request) entry.request = undefined
      })
    entry.request = request
    return request
  }

  const save = (doc: DocId, ui: UiState) => {
    const entry = opened.get(doc)
    if (!entry?.initialized || entry.pending) return
    const version = ++entry.captureVersion
    const names = new Map<GlyphId, string>()
    let missing = false
    for (const tab of ui.tabs) {
      if (tab.doc !== doc || tab.gid === undefined) continue
      const name =
        glyphName(doc, tab.gid) ??
        (entry.generation === generation(doc)
          ? entry.names?.get(tab.gid)
          : undefined)
      if (name !== undefined) names.set(tab.gid, name)
      else if (!tab.textState) missing = true
    }
    const persist = (resolved: ReadonlyMap<GlyphId, string>) => {
      if (!live(doc, entry) || entry.captureVersion !== version) return
      useWorkspaceStore
        .getState()
        .saveCurrent(entry.key, captureWorkspace(doc, ui, resolved))
    }
    if (!missing) persist(names)
    else
      void loadNames(doc, entry).then((resolved) => {
        if (resolved) persist(resolved)
      })
  }

  const saveBackground = (doc: DocId, ui: UiState, previous: UiState) => {
    const entry = opened.get(doc)
    const snapshot = entry && useWorkspaceStore.getState().getCurrent(entry.key)
    if (!snapshot) return
    const previousTabs = previous.tabs.filter((tab) => tab.doc === doc)
    const selected = previousTabs[snapshot.activeTab ?? 0]?.id
    const tabs = ui.tabs.filter((tab) => tab.doc === doc)
    save(doc, {
      ...ui,
      panels: snapshot.panels,
      sizes: snapshot.sizes,
      sections: snapshot.sections,
      textPreviewVisible: snapshot.textPreviewVisible,
      activeTab:
        selected && tabs.some((tab) => tab.id === selected)
          ? selected
          : (tabs[0]?.id ?? null),
    })
  }

  const apply = (
    doc: DocId,
    snapshot: Snapshot,
    names?: ReadonlyMap<GlyphId, string>,
    tabs = true,
    windows = false,
    selectRemembered = true,
  ) => {
    const ui = useUiStore.getState()
    const ids = names
      ? new Map([...names].map(([gid, name]) => [name, gid]))
      : undefined
    const restored = restoreWorkspace(snapshot, doc, ids)
    const active = selectActiveDoc(ui) === doc
    const nextTabs = [] as UiState['tabs']
    let inserted = false
    for (const tab of ui.tabs) {
      if (tab.doc !== doc) nextTabs.push(tab)
      else if (!inserted) {
        nextTabs.push(...restored.tabs)
        inserted = true
      }
    }
    applying = true
    try {
      useUiStore.setState({
        ...(active
          ? {
              panels: restored.panels,
              sizes: restored.sizes,
              sections: restored.sections,
              textPreviewVisible: restored.textPreviewVisible,
            }
          : {}),
        ...(tabs ? { tabs: nextTabs } : {}),
        // Reopening a document restores the tab it was left on. Opening one
        // while another document is already on screen does not: the user asked
        // to see *this* document, so its pinned Overview stays selected and the
        // remembered tabs are restored around it.
        ...(tabs && active && selectRemembered
          ? { activeTab: restored.activeTab }
          : {}),
        // Whatever this document had detached is replaced by what the saved
        // arrangement says; other documents keep their own windows.
        ...(windows
          ? { detached: ui.detached.filter((view) => view.doc !== doc) }
          : {}),
      })
    } finally {
      applying = false
    }
    if (!windows || restored.detached.length === 0) return
    void reopenDetachedViews(restored.detached).then((failures) => {
      // Restoring happens as a document opens, without a user gesture, so a
      // browser may refuse the windows. That is a platform limit, not an error
      // in the document: say it once instead of one error per refused window.
      if (failures.length > 0)
        useUiStore.getState().pushToast({
          kind: 'info',
          message: i18n.t('shellWindow.blocked'),
        })
    })
  }

  const initialize = () => {
    for (const [doc, entry] of opened) {
      if (
        entry.initialized ||
        !useUiStore
          .getState()
          .tabs.some((tab) => tab.doc === doc && tab.kind === 'font')
      )
        continue
      entry.initialized = true
      const snapshot = useWorkspaceStore.getState().getCurrent(entry.key)
      if (!snapshot) {
        save(doc, useUiStore.getState())
        continue
      }
      const alone = !useUiStore.getState().tabs.some((tab) => tab.doc !== doc)
      if (
        !snapshot.tabs.some((tab) => tab.glyphName !== undefined) &&
        !(snapshot.detached ?? []).some((view) => view.kind === 'tab')
      ) {
        apply(doc, snapshot, undefined, true, true, alone)
        continue
      }
      entry.pending = true
      void loadNames(doc, entry).then((names) => {
        if (!live(doc, entry) || !entry.pending) return
        entry.pending = false
        if (
          !useUiStore
            .getState()
            .tabs.some((tab) => tab.doc === doc && tab.kind === 'font')
        )
          return
        apply(doc, snapshot, names, true, true, alone)
      })
    }
  }

  const syncDocuments = () => {
    const docs = useSessionStore.getState().docs
    for (const [doc] of opened)
      if (!docs.some((summary) => summary.doc === doc)) opened.delete(doc)
    for (const summary of docs) {
      const key = workspaceDocumentKey(summary)
      const entry = opened.get(summary.doc)
      if (!entry)
        opened.set(summary.doc, {
          key,
          initialized: false,
          pending: false,
          captureVersion: 0,
        })
      else if (entry.key !== key) {
        useWorkspaceStore.getState().migrateDocumentKey(entry.key, key)
        entry.key = key
      }
    }
    initialize()
  }

  const unsubscribeSession = useSessionStore.subscribe((state, previous) => {
    if (state.docs !== previous.docs) syncDocuments()
  })
  const unsubscribeUi = useUiStore.subscribe((ui, previous) => {
    if (applying || disposed) return
    if (
      ui.tabs === previous.tabs &&
      ui.activeTab === previous.activeTab &&
      !layoutChanged(ui, previous)
    )
      return
    const before = selectActiveDoc(previous)
    const after = selectActiveDoc(ui)
    const pendingBefore = new Set(
      [...opened].filter(([, entry]) => entry.pending).map(([doc]) => doc),
    )
    for (const [doc, entry] of opened) {
      if (
        entry.pending &&
        (tabsChanged(doc, previous, ui) ||
          (before === doc &&
            after === doc &&
            (ui.activeTab !== previous.activeTab ||
              layoutChanged(ui, previous))))
      )
        entry.pending = false
    }
    if (before !== null && before !== after && !pendingBefore.has(before))
      save(before, previous)
    const wasInitialized =
      after === null ? false : opened.get(after)?.initialized
    initialize()
    if (after !== null && before !== after && wasInitialized) {
      const entry = opened.get(after)
      const snapshot =
        entry && useWorkspaceStore.getState().getCurrent(entry.key)
      if (snapshot && entry && !entry.pending)
        apply(after, snapshot, undefined, false)
    }
    for (const [doc, entry] of opened) {
      if (
        doc !== after &&
        doc !== before &&
        entry.initialized &&
        tabsChanged(doc, previous, ui) &&
        ui.tabs.some((tab) => tab.doc === doc) &&
        previous.tabs.some((tab) => tab.doc === doc)
      )
        saveBackground(doc, useUiStore.getState(), previous)
    }
    if (after !== null) save(after, useUiStore.getState())
  })
  syncDocuments()

  return () => {
    if (disposed) return
    const ui = useUiStore.getState()
    const doc = selectActiveDoc(ui)
    if (doc !== null) save(doc, ui)
    disposed = true
    unsubscribeSession()
    unsubscribeUi()
    opened.clear()
  }
}
