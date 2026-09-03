import { create } from 'zustand'

import {
  call,
  getTransport,
  onDocumentEvent,
  type DocId,
  type DocSummary,
} from '../api'
import { i18n } from '../i18n'
import { warningMessage } from '../i18n/warnings'
import { ApiCallError } from '../transport'

import { useDocumentStore } from './documentStore'
import { activeDocId, tabId, useUiStore } from './uiStore'

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
  /** Saves the document; resolves to whether it was written. */
  save: (doc: DocId, asPath?: string) => Promise<boolean>
  /** Asks for a save path (native dialog under Tauri, in-app prompt otherwise). */
  requestSaveAs: (doc: DocId, closeAfterSave?: boolean) => Promise<void>
  /** Saves and, only when the save succeeds, closes the document. */
  saveAndClose: (doc: DocId) => Promise<void>
  close: (doc: DocId) => Promise<void>
  /** Closes, asking Save / Don't Save / Cancel first when the document is dirty. */
  requestClose: (doc: DocId) => Promise<void>
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
    useUiStore.getState().pushToast({
      kind: 'info',
      message: warningMessage(warning, (key, options) => {
        if (!options) return i18n.t(key)
        return i18n.t(key, options)
      }),
    })
}

/** Creates the initial, action-free part of the session state. */
export function initialSessionState(): Pick<
  SessionState,
  'docs' | 'recent' | 'busy'
> {
  return { docs: [], recent: [], busy: false }
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
        if (first && activeDocId() === null) {
          for (const doc of docs)
            useUiStore.getState().openTab({ kind: 'font', doc: doc.doc })
          useUiStore.getState().setActiveTab(tabId('font', first.doc))
        }
      } catch (error) {
        reportError(error)
      }
    })()

    return onDocumentEvent((event) => {
      useDocumentStore.getState().noteGeneration(event.doc, event.generation)
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
      if (!get().docs.some((entry) => entry.doc === doc)) return
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
      set((state) => ({ docs: upsert(state.docs, summary) }))
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
      set((state) => ({ docs: upsert(state.docs, summary) }))
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
      await get().requestSaveAs(doc)
      return false
    }
    try {
      const saved = await call('session.save', { doc, path: asPath ?? null })
      set((state) => ({ docs: upsert(state.docs, saved) }))
      useUiStore.getState().pushToast({
        kind: 'info',
        message: i18n.t('toast.saved', { name: documentLabel(saved) }),
      })
      void get().refreshRecent()
      return true
    } catch (error) {
      reportError(error)
      return false
    }
  },

  requestSaveAs: async (doc, closeAfterSave = false) => {
    if (getTransport().kind === 'tauri') {
      try {
        const { save: saveDialog } = await import('@tauri-apps/plugin-dialog')
        const chosen = await saveDialog({
          filters: [
            {
              name: i18n.t('dialog.saveAs.filter'),
              extensions: ['fwf', 'fwf.gz'],
            },
          ],
        })
        if (typeof chosen !== 'string') return
        const saved = await get().save(doc, chosen)
        if (saved && closeAfterSave) await get().close(doc)
      } catch (error) {
        reportError(error)
      }
      return
    }
    useUiStore.getState().setPathPrompt({ mode: 'saveAs', doc, closeAfterSave })
  },

  saveAndClose: async (doc) => {
    const summary = get().docs.find((entry) => entry.doc === doc)
    if (!isNativeDocumentPath(summary?.path ?? null)) {
      await get().requestSaveAs(doc, true)
      return
    }
    if (await get().save(doc)) await get().close(doc)
  },

  close: async (doc) => {
    try {
      await call('session.close', { doc })
    } catch (error) {
      reportError(error)
      return
    }
    const ui = useUiStore.getState()
    ui.closeTabsForDoc(doc)
    if (ui.closePrompt?.doc === doc) ui.setClosePrompt(null)
    if (ui.pathPrompt?.doc === doc) ui.setPathPrompt(null)
    useDocumentStore.getState().forget(doc)
    set((state) => ({ docs: state.docs.filter((entry) => entry.doc !== doc) }))
  },

  requestClose: async (doc) => {
    const summary = get().docs.find((entry) => entry.doc === doc)
    if (summary?.dirty) {
      useUiStore.getState().setClosePrompt({ doc })
      return
    }
    await get().close(doc)
  },
}))

/** Selector factory: the summary of one document. */
export function selectSummary(doc: DocId | null) {
  return (state: SessionState): DocSummary | null =>
    doc === null
      ? null
      : (state.docs.find((entry) => entry.doc === doc) ?? null)
}

/** The summary of the active document outside React. */
export function activeSummary(): DocSummary | null {
  return selectSummary(activeDocId())(useSessionStore.getState())
}
