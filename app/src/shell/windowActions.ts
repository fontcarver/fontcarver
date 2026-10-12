import type { DocId } from '../api'
import {
  rejoinWindow,
  requestWindow,
  type WindowTarget,
} from '../transport/windows'
import {
  selectActiveTab,
  useUiStore,
  type DetachedView,
  type PanelId,
} from '../stores/uiStore'
import { prepareTabTransition } from './tabActions'

/** Opens or focuses the independent OS window for this document. */
export async function openDocumentWindow(doc: DocId | null): Promise<void> {
  if (doc !== null) await requestWindow({ kind: 'document', doc })
}

/** Moves an editing tab to an independent window after settling its edits. */
export async function detachTab(id: string): Promise<void> {
  const tab = useUiStore.getState().tabs.find((entry) => entry.id === id)
  if (!tab || tab.kind === 'font') return
  const transition = prepareTabTransition()
  if (!(await transition.ready) || !transition.isCurrent()) return
  await requestWindow(
    { kind: 'tab', doc: tab.doc, tab: tab.kind, gid: tab.gid, tabId: tab.id },
    tab,
  )
  if (!(await transition.settle())) return
  useUiStore.getState().addDetached({
    kind: 'tab',
    doc: tab.doc,
    tab: tab.kind,
    ...(tab.gid === undefined ? {} : { gid: tab.gid }),
    tabId: tab.id,
  })
  useUiStore.getState().closeTab(id)
}

/** Detaches the selected editing tab; the Overview remains pinned. */
export async function detachActiveTab(): Promise<void> {
  const id = useUiStore.getState().activeTab
  if (id !== null) await detachTab(id)
}

/** Opens a contextual panel separately and hides the corresponding dock. */
export async function detachPanel(panel: PanelId | 'preview'): Promise<void> {
  const tab = selectActiveTab(useUiStore.getState())
  if (!tab || (panel === 'preview' && tab.kind !== 'glyph')) return
  const transition = prepareTabTransition()
  if (!(await transition.ready) || !transition.isCurrent()) return
  await requestWindow({ kind: 'panel', doc: tab.doc, panel }, tab)
  useUiStore.getState().addDetached({ kind: 'panel', doc: tab.doc, panel })
  if (panel === 'preview') useUiStore.getState().setTextPreviewVisible(false)
  else useUiStore.getState().setPanelVisible(panel, false)
}

/**
 * Reopens the windows a restored workspace says were detached.
 *
 * Each request is independent: a browser that blocks the second popup must not
 * cost the user the first one, so failures are collected and reported together.
 */
export async function reopenDetachedViews(
  views: readonly DetachedView[],
): Promise<unknown[]> {
  const failures: unknown[] = []
  for (const view of views) {
    try {
      await requestWindow(
        view.kind === 'panel'
          ? { kind: 'panel', doc: view.doc, panel: view.panel }
          : {
              kind: 'tab',
              doc: view.doc,
              tab: view.tab,
              ...(view.gid === undefined ? {} : { gid: view.gid }),
              tabId: view.tabId,
            },
      )
      useUiStore.getState().addDetached(view)
    } catch (error) {
      failures.push(error)
    }
  }
  return failures
}

/** Returns the current detached view after all editing participants are saved. */
export async function rejoinDetachedWindow(
  target: WindowTarget,
): Promise<void> {
  const transition = prepareTabTransition()
  if (!(await transition.ready) || !transition.isCurrent()) return
  const tab = selectActiveTab(useUiStore.getState())
  await rejoinWindow(target, tab?.doc === target.doc ? tab : undefined)
}
