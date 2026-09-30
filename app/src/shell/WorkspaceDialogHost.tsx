import { useEffect } from 'react'

import { call, type DocId, type GlyphId } from '../api'
import { WorkspaceDialog } from '../dialogs/WorkspaceDialog'
import { emptyFilter, useDocumentStore } from '../stores/documentStore'
import { useSessionStore } from '../stores/sessionStore'
import { selectActiveDoc, useUiStore } from '../stores/uiStore'
import {
  captureWorkspace,
  restoreWorkspace,
  useWorkspaceStore,
  workspaceDocumentKey,
  type WorkspaceSnapshot,
} from '../stores/workspaceStore'
import { glyphName } from '../views/font/glyphNames'

import { prepareTabTransition } from './tabActions'

async function namesForDocument(doc: DocId): Promise<Map<GlyphId, string>> {
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
    for (const cell of page.items) names.set(cell.gid, cell.name)
    offset += page.items.length
    if (offset >= page.total || page.items.length === 0) return names
  }
}

/** Connects workspace controls to the active document and edit handover. */
export function WorkspaceDialogHost() {
  const open = useWorkspaceStore((state) => state.dialogOpen)
  const close = () => useWorkspaceStore.getState().setDialogOpen(false)
  const doc = useUiStore(selectActiveDoc)
  const summary = useSessionStore((state) =>
    state.docs.find((entry) => entry.doc === doc),
  )
  useEffect(() => {
    if (!summary) useWorkspaceStore.getState().setDialogOpen(false)
  }, [summary])
  if (!open || !summary) return null
  const documentKey = workspaceDocumentKey(summary)

  const capture = async (): Promise<WorkspaceSnapshot> => {
    const ui = useUiStore.getState()
    const names = new Map<GlyphId, string>()
    let missing = false
    for (const tab of ui.tabs.filter((tab) => tab.doc === summary.doc)) {
      if (tab.gid === undefined) continue
      const name = glyphName(summary.doc, tab.gid)
      if (name !== undefined) names.set(tab.gid, name)
      else if (!tab.textState) missing = true
    }
    return captureWorkspace(
      summary.doc,
      ui,
      missing ? await namesForDocument(summary.doc) : names,
    )
  }

  const apply = async (snapshot: WorkspaceSnapshot): Promise<boolean> => {
    const transition = prepareTabTransition()
    if (!(await transition.ready)) return false
    const names = snapshot.tabs.some((tab) => tab.glyphName !== undefined)
      ? await namesForDocument(summary.doc)
      : new Map<GlyphId, string>()
    if (!(await transition.settle()) || !transition.isCurrent()) return false
    const current = useUiStore.getState()
    if (
      !useWorkspaceStore.getState().dialogOpen ||
      selectActiveDoc(current) !== summary.doc ||
      !useSessionStore
        .getState()
        .docs.some((entry) => entry.doc === summary.doc)
    )
      return false
    const restored = restoreWorkspace(
      snapshot,
      summary.doc,
      new Map([...names].map(([gid, name]) => [name, gid])),
    )
    const tabs = [] as typeof current.tabs
    let inserted = false
    for (const tab of current.tabs) {
      if (tab.doc !== summary.doc) tabs.push(tab)
      else if (!inserted) {
        tabs.push(...restored.tabs)
        inserted = true
      }
    }
    useUiStore.setState({ ...restored, tabs })
    return true
  }

  return (
    <WorkspaceDialog
      key={documentKey}
      documentKey={documentKey}
      capture={capture}
      onApply={apply}
      onClose={close}
    />
  )
}
