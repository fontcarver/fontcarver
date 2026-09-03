import { create } from 'zustand'

import {
  call,
  getTransport,
  onDocumentEvent,
  type DocId,
  type DocSummary,
} from '../api'
import { i18n } from '../i18n'
import { ApiCallError } from '../transport'

import { tabId, useUiStore } from './uiStore'

/** File extensions the kernel accepts as native document save targets. */
const NATIVE_EXTENSIONS = ['.fwf', '.fwf.gz']

/** Whether a path is a native FontWorkshop document the kernel can save onto. */
export function isNativeDocumentPath(path: string | null): boolean {
  const lower = path?.toLowerCase() ?? ''
  return NATIVE_EXTENSIONS.some((extension) => lower.endsWith(extension))
}

/** Turns any thrown value into a message suitable for a toast. */
export function errorMessage(error: unknown): string {
  if (error instanceof ApiCallError) return error.message
  if (error instanceof Error) return error.message
  return String(error)
}

/** Human label for a document: its file name, family name, or "Untitled". */
export function documentLabel(summary: DocSummary | undefined): string {
  if (!summary) return i18n.t('document.untitled')
  const name = summary.path?.split(/[\\/]/).pop()
  return name ?? summary.family_name ?? i18n.t('document.untitled')
}

/** Open documents and the kernel session lifecycle. */
export interface SessionState {
  docs: DocSummary[]
  activeDoc: DocId | null
  recent: string[]
  busy: boolean
  start: () => () => void
  refreshRecent: () => Promise<void>
  refreshSummary: (doc: DocId) => Promise<void>
  open: (path: string, faceIndex?: number | null) => Promise<DocSummary | null>
  openDialog: () => Promise<void>
  createDocument: (
    family?: string,
    unitsPerEm?: number,
  ) => Promise<DocSummary | null>
  save: (doc: DocId, asPath?: string) => Promise<void>
  requestSaveAs: (doc: DocId) => void
  close: (doc: DocId) => Promise<void>
  requestClose: (doc: DocId) => Promise<void>
  setActiveDoc: (doc: DocId | null) => void
}

function reportError(error: unknown): void {
  useUiStore
    .getState()
    .pushToast({ kind: 'error', message: errorMessage(error) })
}

function upsert(docs: DocSummary[], summary: DocSummary): DocSummary[] {
  const index = docs.findIndex((doc) => doc.doc === summary.doc)
  if (index < 0) return [...docs, summary]
  const next = [...docs]
  next[index] = summary
  return next
}

function announceWarnings(summary: DocSummary): void {
  for (const warning of summary.warnings)
    useUiStore.getState().pushToast({ kind: 'info', message: warning })
}

/** Creates the initial, action-free part of the session state. */
export function initialSessionState(): Pick<
  SessionState,
  'docs' | 'activeDoc' | 'recent' | 'busy'
> {
  return { docs: [], activeDoc: null, recent: [], busy: false }
}

export const useSessionStore = create<SessionState>((set, get) => ({
  ...initialSessionState(),

  start: () => {
    void get().refreshRecent()
    void (async () => {
      try {
        const docs = await call('session.list', {})
        set({ docs })
        const first = docs[0]
        if (first && get().activeDoc === null) {
          set({ activeDoc: first.doc })
          for (const doc of docs)
            useUiStore.getState().openTab({ kind: 'font', doc: doc.doc })
          useUiStore.getState().setActiveTab(tabId('font', first.doc))
        }
      } catch (error) {
        reportError(error)
      }
    })()

    return onDocumentEvent((event) => {
      set((state) => ({
        docs: state.docs.map((doc) =>
          doc.doc === event.doc
            ? {
                ...doc,
                dirty: event.dirty,
                undo_label: event.undo_label,
                redo_label: event.redo_label,
              }
            : doc,
        ),
      }))
      if (event.change.font_info || event.change.glyph_order)
        void get().refreshSummary(event.doc)
    })
  },

  refreshRecent: async () => {
    try {
      set({ recent: await call('session.recent', {}) })
    } catch (error) {
      reportError(error)
    }
  },

  refreshSummary: async (doc) => {
    try {
      const summary = await call('doc.summary', { doc })
      set((state) => ({ docs: upsert(state.docs, summary) }))
    } catch (error) {
      reportError(error)
    }
  },

  open: async (path, faceIndex) => {
    set({ busy: true })
    try {
      const summary = await call('session.open', {
        path,
        face_index: faceIndex ?? null,
      })
      set((state) => ({
        docs: upsert(state.docs, summary),
        activeDoc: summary.doc,
      }))
      useUiStore.getState().openTab({ kind: 'font', doc: summary.doc })
      announceWarnings(summary)
      void get().refreshRecent()
      return summary
    } catch (error) {
      reportError(error)
      return null
    } finally {
      set({ busy: false })
    }
  },

  openDialog: async () => {
    if (getTransport().kind === 'tauri') {
      try {
        const { open } = await import('@tauri-apps/plugin-dialog')
        const selected = await open({ multiple: false, directory: false })
        if (typeof selected === 'string') await get().open(selected)
      } catch (error) {
        reportError(error)
      }
      return
    }
    useUiStore.getState().setPathPrompt({ mode: 'open' })
  },

  createDocument: async (family, unitsPerEm) => {
    try {
      const summary = await call('session.new', {
        family: family ?? i18n.t('document.newFamily'),
        units_per_em: unitsPerEm ?? 1000,
      })
      set((state) => ({
        docs: upsert(state.docs, summary),
        activeDoc: summary.doc,
      }))
      useUiStore.getState().openTab({ kind: 'font', doc: summary.doc })
      return summary
    } catch (error) {
      reportError(error)
      return null
    }
  },

  save: async (doc, asPath) => {
    const summary = get().docs.find((entry) => entry.doc === doc)
    if (asPath === undefined && !isNativeDocumentPath(summary?.path ?? null)) {
      get().requestSaveAs(doc)
      return
    }
    try {
      const saved = await call('session.save', { doc, path: asPath ?? null })
      set((state) => ({ docs: upsert(state.docs, saved) }))
      useUiStore.getState().pushToast({
        kind: 'info',
        message: i18n.t('toast.saved', { name: documentLabel(saved) }),
      })
      void get().refreshRecent()
    } catch (error) {
      reportError(error)
    }
  },

  requestSaveAs: (doc) => {
    useUiStore.getState().setPathPrompt({ mode: 'saveAs', doc })
  },

  close: async (doc) => {
    try {
      await call('session.close', { doc })
    } catch (error) {
      reportError(error)
      return
    }
    useUiStore.getState().closeTabsForDoc(doc)
    set((state) => {
      const docs = state.docs.filter((entry) => entry.doc !== doc)
      return {
        docs,
        activeDoc:
          state.activeDoc === doc
            ? (docs[docs.length - 1]?.doc ?? null)
            : state.activeDoc,
      }
    })
  },

  requestClose: async (doc) => {
    const summary = get().docs.find((entry) => entry.doc === doc)
    if (summary?.dirty) {
      const confirmed = globalThis.confirm?.(
        i18n.t('dialog.closeDirty', { name: documentLabel(summary) }),
      )
      if (!confirmed) return
    }
    await get().close(doc)
  },

  setActiveDoc: (activeDoc) => set({ activeDoc }),
}))

/** Selector: the summary of the active document, if one is open. */
export function selectActiveSummary(state: SessionState): DocSummary | null {
  return state.docs.find((doc) => doc.doc === state.activeDoc) ?? null
}
