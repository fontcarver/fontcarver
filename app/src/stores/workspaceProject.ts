import { call, type DocId } from '../api'
import { i18n } from '../i18n'
import { readWindowTarget } from '../transport/windows'

import { useSessionStore } from './sessionStore'
import { reportError, useUiStore } from './uiStore'
import { readWorkspaceSnapshot } from './workspaceSnapshot'
import {
  useWorkspaceStore,
  workspaceDocumentKey,
  type WorkspacePreset,
  type WorkspaceStorage,
} from './workspaceStore'

export type { WorkspaceStorage } from './workspaceStore'

const storage = new Map<string, WorkspaceStorage>()
/** Last value each key was synchronised with, so a push is never a loop. */
const synced = new Map<string, string>()

/** Where the named workspaces of the document with this key are stored. */
export function workspaceStorageOf(key: string): WorkspaceStorage {
  return storage.get(key) ?? 'local'
}

/** Records the answer in the store as well, so open dialogs re-render. */
function remember(key: string, kind: WorkspaceStorage | null): void {
  if (kind === null) storage.delete(key)
  else storage.set(key, kind)
  useWorkspaceStore.getState().setStorage(key, kind)
}

function isStored(
  workspaces: readonly { name: string }[],
  name: string,
): boolean {
  return workspaces.some((entry) => entry.name === name)
}

function serialise(presets: readonly WorkspacePreset[]): string {
  return JSON.stringify(
    presets.map((preset) => ({ name: preset.name, layout: preset.snapshot })),
  )
}

/**
 * Keeps each document's named workspaces in the project it belongs to.
 *
 * The store remains the single surface the dialog talks to; this module only
 * loads a document's stored arrangements into it when the document opens and
 * writes them back when the user saves or deletes one. Records the kernel
 * rejects as unreadable are reported once and the local records are kept, so a
 * damaged sidecar costs a message rather than a workspace.
 */
export function installWorkspaceProjectSync(): () => void {
  // A detached view is not a document window: its shell holds one transferred
  // tab, so it must not read or write the document's saved arrangements.
  const target = readWindowTarget()
  if (target && target.kind !== 'document') return () => {}
  let stopped = false
  const pending = new Set<string>()
  // Keys whose stored records are being merged in. Saving them into the store
  // must not look like a user change and write the same bytes straight back.
  const hydrating = new Set<string>()

  const load = (doc: DocId, key: string) => {
    if (pending.has(key)) return
    pending.add(key)
    void call('session.workspaces', { doc }).then(
      (result) => {
        pending.delete(key)
        if (stopped) return
        if (result.path === null) {
          remember(key, 'local')
          return
        }
        remember(key, 'project')
        hydrating.add(key)
        const store = useWorkspaceStore.getState()
        const local = store.documents[key]?.presets ?? []
        const stored: WorkspacePreset[] = []
        for (const entry of result.workspaces) {
          const snapshot = readWorkspaceSnapshot(entry.layout)
          if (snapshot) stored.push({ name: entry.name, snapshot })
        }
        // The project is authoritative for a name it holds; a workspace saved
        // while the document had no project keeps its place alongside.
        for (const preset of local)
          if (!stored.some((entry) => entry.name === preset.name))
            stored.push(preset)
        for (const preset of stored)
          store.savePreset(key, preset.name, preset.snapshot)
        // A workspace saved while the document had no project joins it now;
        // otherwise the file already holds what the store now holds and must
        // not be rewritten, so that opening a document is never a Git change.
        const joining = local.some(
          (preset) => !isStored(result.workspaces, preset.name),
        )
        if (!joining)
          synced.set(
            key,
            serialise(
              useWorkspaceStore.getState().documents[key]?.presets ?? [],
            ),
          )
        hydrating.delete(key)
        if (joining) push(doc, key)
      },
      (error: unknown) => {
        pending.delete(key)
        hydrating.delete(key)
        if (stopped) return
        remember(key, 'local')
        useUiStore.getState().pushToast({
          kind: 'error',
          message: i18n.t('workspace.projectError', {
            message: error instanceof Error ? error.message : String(error),
          }),
        })
      },
    )
  }

  const push = (doc: DocId, key: string) => {
    if (storage.get(key) !== 'project' || hydrating.has(key)) return
    const presets = useWorkspaceStore.getState().documents[key]?.presets ?? []
    const value = serialise(presets)
    if (synced.get(key) === value) return
    synced.set(key, value)
    void call('session.set_workspaces', {
      doc,
      workspaces: presets.map((preset) => ({
        name: preset.name,
        layout: preset.snapshot as unknown,
      })),
    }).catch((error: unknown) => {
      if (stopped) return
      // Leave the value unsynchronised so the next change retries.
      synced.delete(key)
      reportError(error)
    })
  }

  const keys = () =>
    new Map(
      useSessionStore
        .getState()
        .docs.map((summary) => [workspaceDocumentKey(summary), summary.doc]),
    )

  const syncDocuments = () => {
    const open = keys()
    for (const key of [...storage.keys()])
      if (!open.has(key)) {
        remember(key, null)
        synced.delete(key)
      }
    for (const [key, doc] of open) if (!storage.has(key)) load(doc, key)
  }

  const stopSession = useSessionStore.subscribe((state, previous) => {
    if (state.docs !== previous.docs) syncDocuments()
  })
  const stopWorkspace = useWorkspaceStore.subscribe((state, previous) => {
    if (state.documents === previous.documents) return
    for (const [key, doc] of keys()) push(doc, key)
  })
  syncDocuments()

  return () => {
    stopped = true
    stopSession()
    stopWorkspace()
    for (const key of [...storage.keys()])
      useWorkspaceStore.getState().setStorage(key, null)
    storage.clear()
    synced.clear()
    pending.clear()
    hydrating.clear()
  }
}
