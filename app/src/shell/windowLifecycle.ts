import { call, type DocId } from '../api'
import { activeEditor } from '../editor/activeEditor'
import { hasPendingDocumentDraft } from '../editor/documentEditParticipants'
import { settleDocumentEdits } from '../editor/documentPersistence'
import {
  documentEditRevision,
  pendingDocumentEdits,
} from '../editor/editSession'
import { useDocumentStore } from '../stores/documentStore'
import { useEditorStore } from '../stores/editorStore'
import { useSessionStore } from '../stores/sessionStore'
import { reportError, tabId, useUiStore, type Tab } from '../stores/uiStore'
import {
  closeCurrentWindow,
  installWindowCloseGuard,
  isDesktopWindow,
  onWindowClosed,
  onWindowRejoin,
  readWindowTab,
  readWindowTarget,
  requestWindow,
  writeWindowTab,
  type WindowTarget,
} from '../transport/windows'
import { prepareTabTransition } from './tabActions'

function restoredTab(target: WindowTarget): Tab | null {
  const payload = readWindowTab(target)
  if (payload?.doc === target.doc) return payload
  if (target.kind !== 'tab') return null
  return {
    id: target.tabId ?? tabId(target.tab, target.doc, target.gid),
    kind: target.tab,
    doc: target.doc,
    ...(target.gid === undefined ? {} : { gid: target.gid }),
  }
}

/** Preserves a text tab's identity when its active glyph has since changed. */
function restoreTab(tab: Tab, activate = true): void {
  useUiStore.setState((state) => ({
    tabs: [...state.tabs.filter((entry) => entry.id !== tab.id), tab],
    activeTab: activate ? tab.id : state.activeTab,
  }))
}

/**
 * Binds one shell to its shared-session document and platform window. Native
 * closes use the same draft/save barriers as File Close; detached views only
 * finish their drafts and leave the document owned by its document window.
 */
export function installWindowLifecycle(): () => void {
  const target = readWindowTarget()
  const desktop = isDesktopWindow()
  const scoped = target !== null || desktop
  let scope: DocId | null =
    target?.doc ?? useUiStore.getState().tabs[0]?.doc ?? null
  let initialized = false
  let stopped = false
  let reconciling = false
  let confirming = false
  let closeIntent: DocId | null = null
  let closing = false
  let refreshing = false
  const routing = new Set<DocId>()

  const closeWindow = () => {
    if (stopped || closing) return
    closing = true
    void closeCurrentWindow().catch((error: unknown) => {
      closing = false
      reportError(error)
    })
  }

  const writePayload = () => {
    if (!target || target.kind === 'document') return
    const ui = useUiStore.getState()
    const tab =
      target.kind === 'tab'
        ? ui.tabs.find(
            (entry) =>
              entry.id ===
              (target.tabId ?? tabId(target.tab, target.doc, target.gid)),
          )
        : ui.tabs.find(
            (entry) => entry.id === ui.activeTab && entry.doc === target.doc,
          )
    if (tab) writeWindowTab(target, tab)
  }

  const reconcile = () => {
    if (stopped || reconciling || !scoped) return
    reconciling = true
    try {
      const docs = useSessionStore.getState().docs
      if (!initialized) {
        if (target) {
          if (!docs.some((doc) => doc.doc === target.doc)) return
          const overview: Tab = {
            id: tabId('font', target.doc),
            kind: 'font',
            doc: target.doc,
          }
          const restored = restoredTab(target)
          const tabs =
            restored && restored.id !== overview.id
              ? [overview, restored]
              : [overview]
          useUiStore.setState({ tabs, activeTab: restored?.id ?? overview.id })
        } else {
          scope ??= docs[0]?.doc ?? null
          if (scope === null) return
          if (!useUiStore.getState().tabs.some((tab) => tab.doc === scope))
            useUiStore.getState().openTab({ kind: 'font', doc: scope })
        }
        initialized = true
      }
      const ui = useUiStore.getState()
      if (!target && scope === null) scope = ui.tabs[0]?.doc ?? null
      for (const doc of new Set(ui.tabs.map((tab) => tab.doc))) {
        if (doc === scope || routing.has(doc)) continue
        const tab =
          ui.tabs.find(
            (entry) => entry.doc === doc && entry.id === ui.activeTab,
          ) ?? ui.tabs.find((entry) => entry.doc === doc)
        routing.add(doc)
        void requestWindow({ kind: 'document', doc }, tab)
          .then(async () => {
            await settleDocumentEdits(doc)
            if (!stopped && doc !== scope)
              useUiStore.getState().closeTabsForDoc(doc)
          })
          .catch(reportError)
          .finally(() => routing.delete(doc))
      }
      writePayload()
    } catch (error) {
      reportError(error)
    } finally {
      reconciling = false
    }
  }

  const restoreDetached = (returned: WindowTarget) => {
    if (stopped || returned.kind === 'document') return
    // Detached views are never owners of another detached view.
    if (target && target.kind !== 'document') return
    if (scoped && returned.doc !== scope) return
    if (
      !useSessionStore.getState().docs.some((doc) => doc.doc === returned.doc)
    )
      return
    // The window is gone either way, so the workspace must stop listing it.
    useUiStore.getState().removeDetached(
      returned.kind === 'tab'
        ? {
            kind: 'tab',
            doc: returned.doc,
            tab: returned.tab,
            ...(returned.gid === undefined ? {} : { gid: returned.gid }),
            tabId:
              returned.tabId ?? tabId(returned.tab, returned.doc, returned.gid),
          }
        : { kind: 'panel', doc: returned.doc, panel: returned.panel },
    )
    if (returned.kind === 'tab') {
      const tab = restoredTab(returned)
      if (tab) {
        const transition = prepareTabTransition()
        restoreTab(tab, false)
        void transition.ready.then((ready) => {
          if (ready && !stopped && transition.isCurrent())
            useUiStore.getState().setActiveTab(tab.id)
        })
      }
    } else if (returned.panel === 'preview') {
      useUiStore.getState().setTextPreviewVisible(true)
    } else {
      useUiStore.getState().setPanelVisible(returned.panel, true)
    }
  }

  /**
   * Publishes this window's active tab to its detached panels.
   *
   * A detached sidebar or Inspector is a view of whatever this window is
   * editing, so when the active tab moves to another glyph the panel has to
   * follow. Only the owning window publishes; the panel windows read.
   */
  const publishPanels = () => {
    if (stopped || target) return
    const ui = useUiStore.getState()
    const active = ui.tabs.find((entry) => entry.id === ui.activeTab)
    if (!active) return
    for (const view of ui.detached) {
      if (view.kind !== 'panel' || view.doc !== active.doc) continue
      try {
        writeWindowTab(
          { kind: 'panel', doc: view.doc, panel: view.panel },
          active,
        )
      } catch (error) {
        // A tab this target cannot carry is not worth interrupting editing for.
        void error
      }
    }
  }

  const stopUi = useUiStore.subscribe((state, previous) => {
    if (state.tabs !== previous.tabs) reconcile()
    if (
      state.tabs !== previous.tabs ||
      state.activeTab !== previous.activeTab ||
      state.detached !== previous.detached
    )
      publishPanels()
  })
  const stopSession = useSessionStore.subscribe((state, previous) => {
    if (state.docs === previous.docs) return
    for (const doc of previous.docs) {
      if (state.docs.some((entry) => entry.doc === doc.doc)) continue
      useUiStore.getState().closeTabsForDoc(doc.doc)
      useDocumentStore.getState().forget(doc.doc)
      if (doc.doc === scope) {
        if ((target || closeIntent === doc.doc) && !confirming) closeWindow()
        if (!target) scope = null
      }
    }
    reconcile()
  })
  /**
   * Adopts the tab payload the owning window publishes.
   *
   * A detached panel is a view of whatever the owner is editing, so when the
   * owner steps to another glyph this window has to follow. The owner writes its
   * current tab to storage on every tab change; a storage event is the one
   * cross-window signal a browser gives us, and the native shell writes through
   * the same store.
   */
  const adoptPayload = () => {
    if (stopped || !target || target.kind === 'document') return
    const tab = restoredTab(target)
    if (!tab) return
    const ui = useUiStore.getState()
    const current = ui.tabs.find((entry) => entry.id === ui.activeTab)
    if (current && current.id === tab.id && current.gid === tab.gid) return
    const overview: Tab = {
      id: tabId('font', target.doc),
      kind: 'font',
      doc: target.doc,
    }
    useUiStore.setState({
      tabs: tab.id === overview.id ? [overview] : [overview, tab],
      activeTab: tab.id,
    })
  }

  const onStorage = (event: StorageEvent) => {
    if (event.key !== null && !event.key.startsWith('fontcarver:window-tab:'))
      return
    adoptPayload()
  }
  if (target && target.kind !== 'document')
    window.addEventListener('storage', onStorage)

  const stopRejoin = onWindowRejoin(restoreDetached)
  const stopClosed = onWindowClosed(restoreDetached)
  const stopGuard = installWindowCloseGuard({
    isDirty: () => {
      const relevant = useSessionStore
        .getState()
        .docs.filter((doc) => !scoped || doc.doc === scope)
      return relevant.some(
        (doc) =>
          doc.dirty ||
          hasPendingDocumentDraft(doc.doc) ||
          pendingDocumentEdits(doc.doc) !== undefined ||
          (activeEditor()?.documentId === doc.doc &&
            useEditorStore.getState().editing),
      )
    },
    confirmClose: async () => {
      if (target && target.kind !== 'document') {
        await settleDocumentEdits(target.doc)
        writePayload()
        return true
      }
      const docs = useSessionStore
        .getState()
        .docs.filter((doc) => !scoped || doc.doc === scope)
      confirming = true
      try {
        for (const doc of docs) {
          closeIntent = doc.doc
          await useSessionStore.getState().requestClose(doc.doc)
          if (
            useSessionStore
              .getState()
              .docs.some((entry) => entry.doc === doc.doc)
          )
            return false
        }
        return true
      } finally {
        confirming = false
      }
    },
    onError: reportError,
  })

  const refresh = async () => {
    if (stopped || refreshing) return
    refreshing = true
    const before = new Map(
      useSessionStore.getState().docs.map((doc) => [doc.doc, doc]),
    )
    const revisions = new Map(
      [...before.keys()].map((doc) => [doc, documentEditRevision(doc)]),
    )
    try {
      const listed = await call('session.list', {})
      if (stopped) return
      const current = useSessionStore.getState().docs
      const currentById = new Map(current.map((doc) => [doc.doc, doc]))
      // A focus request must not resurrect a local close, lose a newly opened
      // document, or overwrite an edit event received while the list was read.
      const docs = listed
        .filter((doc) => !before.has(doc.doc) || currentById.has(doc.doc))
        .map((doc) => {
          const latest = currentById.get(doc.doc)
          return latest &&
            (latest !== before.get(doc.doc) ||
              documentEditRevision(doc.doc) !== revisions.get(doc.doc))
            ? latest
            : doc
        })
      for (const doc of current)
        if (
          !before.has(doc.doc) &&
          !docs.some((entry) => entry.doc === doc.doc)
        )
          docs.push(doc)
      useSessionStore.setState({ docs })
      if (target && !docs.some((doc) => doc.doc === target.doc)) closeWindow()
    } catch (error) {
      reportError(error)
    } finally {
      refreshing = false
    }
  }
  const onFocus = () => {
    void refresh()
  }
  window.addEventListener('focus', onFocus)
  reconcile()
  return () => {
    stopped = true
    stopUi()
    stopSession()
    stopRejoin()
    stopClosed()
    stopGuard()
    window.removeEventListener('storage', onStorage)
    window.removeEventListener('focus', onFocus)
  }
}
