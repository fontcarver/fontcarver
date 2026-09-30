import { create } from 'zustand'

import type { DocSummary } from '../api'

import {
  copyWorkspace,
  readWorkspaceSnapshot,
  workspaceRecord,
  type WorkspaceSnapshot,
} from './workspaceSnapshot'

export { captureWorkspace, restoreWorkspace } from './workspaceSnapshot'
export type {
  WorkspaceSnapshot,
  WorkspaceTab,
  WorkspaceUi,
} from './workspaceSnapshot'

/** Local storage location and schema version for workspace records. */
export const WORKSPACE_STORAGE_KEY = 'fontcarver.workspaces'
const MAX_DOCUMENTS = 100
const MAX_PRESETS = 32

/** A user-named layout saved alongside the document's latest arrangement. */
export interface WorkspacePreset {
  name: string
  snapshot: WorkspaceSnapshot
}

/** Latest automatic snapshot and named snapshots for a single document. */
export interface DocumentWorkspace {
  current: WorkspaceSnapshot | null
  presets: WorkspacePreset[]
}

/** Persistent state and operations for document-local workspaces. */
export interface WorkspaceState {
  dialogOpen: boolean
  setDialogOpen: (open: boolean) => void
  documents: Record<string, DocumentWorkspace>
  saveCurrent: (key: string, snapshot: WorkspaceSnapshot) => void
  getCurrent: (key: string) => WorkspaceSnapshot | null
  savePreset: (
    key: string,
    name: string,
    snapshot: WorkspaceSnapshot,
  ) => boolean
  applyPreset: (key: string, name: string) => WorkspaceSnapshot | null
  deletePreset: (key: string, name: string) => void
  migrateDocumentKey: (from: string, to: string) => void
}

/** Reopening the same path uses its workspace even when session IDs change. */
export function workspaceDocumentKey(
  document: Pick<DocSummary, 'doc' | 'path'>,
): string {
  return document.path ? `path:${document.path}` : `doc:${document.doc}`
}

function readDocument(value: unknown): DocumentWorkspace | null {
  const data = workspaceRecord(value)
  if (!data) return null
  const current = readWorkspaceSnapshot(data.current)
  const presets: WorkspacePreset[] = []
  if (Array.isArray(data.presets))
    for (const raw of data.presets.slice(0, MAX_PRESETS)) {
      const preset = workspaceRecord(raw)
      if (!preset || typeof preset.name !== 'string') continue
      const name = preset.name.trim()
      const snapshot = readWorkspaceSnapshot(preset.snapshot)
      if (
        name &&
        name.length <= 80 &&
        snapshot &&
        !presets.some((entry) => entry.name === name)
      )
        presets.push({ name, snapshot })
    }
  return current || presets.length ? { current, presets } : null
}

/** Loads only supported, validated records; unavailable storage stays usable. */
export function initialWorkspaceState(): Pick<
  WorkspaceState,
  'documents' | 'dialogOpen'
> {
  try {
    const raw = globalThis.localStorage?.getItem(WORKSPACE_STORAGE_KEY)
    const parsed = raw ? workspaceRecord(JSON.parse(raw)) : null
    if (parsed?.version !== 1) return { documents: {}, dialogOpen: false }
    const documents = Object.fromEntries(
      Object.entries(workspaceRecord(parsed.documents) ?? {})
        .slice(-MAX_DOCUMENTS)
        .flatMap(([key, value]) => {
          const workspace = readDocument(value)
          return workspace && key.length <= 16_384 ? [[key, workspace]] : []
        }),
    )
    return { documents, dialogOpen: false }
  } catch {
    return { documents: {}, dialogOpen: false }
  }
}

function writeDocuments(documents: WorkspaceState['documents']): void {
  try {
    // Unsaved document IDs belong to this session only and must not leak to
    // unrelated new documents after the application restarts.
    const persistent = Object.fromEntries(
      Object.entries(documents).filter(([key]) => !key.startsWith('doc:')),
    )
    globalThis.localStorage?.setItem(
      WORKSPACE_STORAGE_KEY,
      JSON.stringify({ version: 1, documents: persistent }),
    )
  } catch {
    // Disabled storage or an exhausted quota leaves the session functional.
  }
}

export const useWorkspaceStore = create<WorkspaceState>((set, get) => {
  const refreshedDocuments = (): WorkspaceState['documents'] => ({
    ...get().documents,
    ...initialWorkspaceState().documents,
  })
  const document = (
    key: string,
    documents = get().documents,
  ): DocumentWorkspace =>
    Object.hasOwn(documents, key)
      ? documents[key]
      : { current: null, presets: [] }
  const update = (
    key: string,
    change: (saved: DocumentWorkspace) => DocumentWorkspace,
  ) => {
    // Each window reads the latest persisted records before updating its own
    // document so a stale in-memory copy cannot erase another window's work.
    const latest = refreshedDocuments()
    const workspace = change(document(key, latest))
    const entries = Object.entries(latest).filter(
      ([existing]) => existing !== key,
    )
    const documents = Object.fromEntries(
      [...entries, [key, workspace]].slice(-MAX_DOCUMENTS),
    )
    set({ documents })
    writeDocuments(documents)
  }
  return {
    ...initialWorkspaceState(),
    setDialogOpen: (dialogOpen) => set({ dialogOpen }),
    saveCurrent: (key, snapshot) =>
      update(key, (saved) => ({ ...saved, current: copyWorkspace(snapshot) })),
    getCurrent: (key) => {
      const current = document(key).current
      return current ? copyWorkspace(current) : null
    },
    savePreset: (key, rawName, snapshot) => {
      const name = rawName.trim()
      const saved = document(key, refreshedDocuments())
      if (
        !name ||
        name.length > 80 ||
        (saved.presets.length >= MAX_PRESETS &&
          !saved.presets.some((preset) => preset.name === name))
      )
        return false
      const presets = [
        ...saved.presets.filter((preset) => preset.name !== name),
        { name, snapshot: copyWorkspace(snapshot) },
      ]
      update(key, (latest) => ({ ...latest, presets }))
      return true
    },
    applyPreset: (key, name) => {
      const snapshot = document(key, refreshedDocuments()).presets.find(
        (preset) => preset.name === name,
      )?.snapshot
      if (!snapshot) return null
      get().saveCurrent(key, snapshot)
      return copyWorkspace(snapshot)
    },
    deletePreset: (key, name) => {
      update(key, (saved) => ({
        ...saved,
        presets: saved.presets.filter((preset) => preset.name !== name),
      }))
    },
    migrateDocumentKey: (from, to) => {
      const documents = refreshedDocuments()
      if (from === to || !Object.hasOwn(documents, from)) return
      const previous = document(from, documents)
      const destination = document(to, documents)
      delete documents[from]
      documents[to] = {
        current: previous.current ?? destination.current,
        presets: [
          ...destination.presets.filter(
            (preset) =>
              !previous.presets.some((entry) => entry.name === preset.name),
          ),
          ...previous.presets,
        ].slice(-MAX_PRESETS),
      }
      set({ documents })
      writeDocuments(documents)
    },
  }
})
