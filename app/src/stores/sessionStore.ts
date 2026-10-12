import { create } from 'zustand'

import { call, onDocumentEvent, type DocId, type DocSummary } from '../api'
import { i18n } from '../i18n'
import { warningMessage } from '../i18n/warnings'
import {
  choosePath,
  hasNativeFileDialog,
  OPEN_FONT_EXTENSIONS,
} from '../shell/nativeDialogs'
import { withSettledDocument } from '../editor/documentPersistence'
import {
  flushDocumentEditParticipants,
  suspendDocumentEditParticipants,
} from '../editor/documentEditParticipants'
import { documentEditRevision } from '../editor/editSession'
import { prepareTabTransition } from '../shell/tabActions'

import { useDocumentStore } from './documentStore'
import { activeDocId, reportError, useUiStore } from './uiStore'

/** File extensions the kernel accepts as native document save targets. */
// `.fwf` is the pre-rename extension; the kernel still saves onto it.
const NATIVE_EXTENSIONS = ['.fcf', '.fcf.gz', '.fcfp', '.fwf', '.fwf.gz']

/** Whether a path is a native FontCarver document the kernel can save onto. */
export function isNativeDocumentPath(path: string | null): boolean {
  const lower = path?.toLowerCase() ?? ''
  return NATIVE_EXTENSIONS.some((extension) => lower.endsWith(extension))
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
  /** Opens a directory project through a folder picker or browser path prompt. */
  openProjectDialog: () => Promise<void>
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
  /** Opens one crash-recovery snapshot as a dirty document in a new tab. */
  recover: (autosavePath: string) => Promise<DocSummary | null>
  /** Deletes one crash-recovery snapshot; resolves to whether it was removed. */
  discardRecovery: (autosavePath: string) => Promise<boolean>
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

/** Called inside the document barrier after the close decision is made. */
async function closeDocument(doc: DocId): Promise<void> {
  await call('session.close', { doc })
  const ui = useUiStore.getState()
  ui.closeTabsForDoc(doc)
  if (ui.closePrompt?.doc === doc) ui.setClosePrompt(null)
  if (ui.pathPrompt?.doc === doc) ui.setPathPrompt(null)
  useDocumentStore.getState().forget(doc)
  useSessionStore.setState((state) => ({
    docs: state.docs.filter((entry) => entry.doc !== doc),
  }))
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
          // Activate the first document once. Workspace subscriptions may
          // restore its editing tab synchronously while Overview is opened.
          for (const doc of docs)
            useUiStore
              .getState()
              .openTab({ kind: 'font', doc: doc.doc }, doc.doc === first.doc)
        }
      } catch (error) {
        reportError(error)
      }
    })()

    return onDocumentEvent((event) => {
      useDocumentStore.getState().noteGeneration(event.doc, event.generation)
      // A fresh edit always clears the redo stack, so a new undo label with no
      // redo label behind it is an action that just happened: the status bar
      // flashes it (design spec §10). Undo and redo flash from their commands,
      // which know which entry they moved.
      const previous = get().docs.find((doc) => doc.doc === event.doc)
      if (
        event.redo_label === null &&
        event.undo_label !== null &&
        event.undo_label !== previous?.undo_label
      )
        useUiStore.getState().flashUndoLabel(event.undo_label)
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
    // Opening a path that is already open deliberately opens it again: the
    // kernel keeps the two documents apart, and comparing a file with the
    // version you have edited is an accepted workflow (see the P2a kerning
    // handover case). Each document gets exactly one pinned Overview; two
    // documents of one file therefore show two, named after the same file.
    set({ busy: true })
    const transition = prepareTabTransition()
    try {
      const summary = await call('session.open', {
        path,
        face_index: faceIndex ?? null,
      })
      set((state) => ({ docs: upsert(state.docs, summary) }))
      const ready = (await transition.ready) && (await transition.settle())
      useUiStore
        .getState()
        .openTab(
          { kind: 'font', doc: summary.doc },
          ready && transition.isCurrent(),
        )
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
    // No native dialog: ask for the path in the application itself, without
    // waiting on a platform round trip that cannot happen here.
    if (!hasNativeFileDialog()) {
      useUiStore.getState().setPathPrompt({ mode: 'open' })
      return
    }
    try {
      const choice = await choosePath({
        kind: 'open',
        filterName: i18n.t('dialog.open.filter'),
        extensions: OPEN_FONT_EXTENSIONS,
      })
      if (choice.kind === 'chosen') await get().open(choice.path)
    } catch (error) {
      reportError(error)
    }
  },

  openProjectDialog: async () => {
    if (!hasNativeFileDialog()) {
      useUiStore.getState().setPathPrompt({ mode: 'open' })
      return
    }
    try {
      const choice = await choosePath({ kind: 'open', directory: true })
      if (choice.kind === 'chosen') await get().open(choice.path)
    } catch (error) {
      reportError(error)
    }
  },

  createDocument: async (family, unitsPerEm) => {
    const transition = prepareTabTransition()
    try {
      const summary = await call('session.new', {
        family: family ?? i18n.t('document.newFamily'),
        units_per_em: unitsPerEm ?? 1000,
      })
      set((state) => ({ docs: upsert(state.docs, summary) }))
      const ready = (await transition.ready) && (await transition.settle())
      useUiStore
        .getState()
        .openTab(
          { kind: 'font', doc: summary.doc },
          ready && transition.isCurrent(),
        )
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
      const saved = await withSettledDocument(doc, () =>
        call('session.save', { doc, path: asPath ?? null }),
      )
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
    if (!hasNativeFileDialog()) {
      useUiStore
        .getState()
        .setPathPrompt({ mode: 'saveAs', doc, closeAfterSave })
      return
    }
    try {
      const choice = await choosePath({
        kind: 'save',
        filterName: i18n.t('dialog.saveAs.filter'),
        extensions: ['fcf', 'fcf.gz', 'fcfp'],
      })
      if (choice.kind !== 'chosen') return
      const saved = await get().save(doc, choice.path)
      if (saved && closeAfterSave) await get().requestClose(doc)
    } catch (error) {
      reportError(error)
    }
  },

  saveAndClose: async (doc) => {
    const summary = get().docs.find((entry) => entry.doc === doc)
    if (!isNativeDocumentPath(summary?.path ?? null)) {
      await get().requestSaveAs(doc, true)
      return
    }
    if (await get().save(doc)) await get().requestClose(doc)
  },

  close: async (doc) => {
    try {
      await withSettledDocument(doc, async () => {
        const resume = suspendDocumentEditParticipants(doc)
        try {
          await flushDocumentEditParticipants(doc)
          await closeDocument(doc)
        } finally {
          resume()
        }
      })
    } catch (error) {
      reportError(error)
      return
    }
  },

  requestClose: async (doc) => {
    try {
      await withSettledDocument(doc, async () => {
        const resume = suspendDocumentEditParticipants(doc)
        try {
          for (;;) {
            await flushDocumentEditParticipants(doc)
            const revision = documentEditRevision(doc)
            // The last edit event may still be in transit, and a draft staged
            // during an earlier read must be flushed before trusting clean.
            const summary = await call('doc.summary', { doc })
            if (documentEditRevision(doc) !== revision) continue
            set((state) => ({ docs: upsert(state.docs, summary) }))
            if (summary.dirty) useUiStore.getState().setClosePrompt({ doc })
            else await closeDocument(doc)
            return
          }
        } finally {
          resume()
        }
      })
    } catch (error) {
      reportError(error)
    }
  },

  recover: async (autosavePath) => {
    const transition = prepareTabTransition()
    try {
      const summary = await call('session.recover', {
        autosave_path: autosavePath,
      })
      set((state) => ({ docs: upsert(state.docs, summary) }))
      const ready = (await transition.ready) && (await transition.settle())
      useUiStore
        .getState()
        .openTab(
          { kind: 'font', doc: summary.doc },
          ready && transition.isCurrent(),
        )
      announceWarnings(summary)
      return summary
    } catch (error) {
      reportError(error)
      return null
    }
  },

  discardRecovery: async (autosavePath) => {
    try {
      await call('session.discard_recovery', { autosave_path: autosavePath })
      return true
    } catch (error) {
      reportError(error)
      return false
    }
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
