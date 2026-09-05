import { create } from 'zustand'

import { onProgressEvent, type DocId } from '../api'

import { useUiStore } from './uiStore'

/** Stage identifiers the kernel reports while an operation runs. */
export const COMPILE_STAGE = 'compile'
export const PRESERVE_STAGE = 'preserve'
export const WRITE_STAGE = 'write'
export const FAILED_STAGE = 'failed'

/** How far one long-running kernel operation has got. */
export interface Progress {
  readonly stage: string
  readonly percent: number
}

/**
 * Whether an event ends the stream. Every operation terminates in either
 * `write 100` or `failed 100`, so the line always disappears on its own.
 */
function isTerminal(stage: string, percent: number): boolean {
  return percent >= 100 && (stage === WRITE_STAGE || stage === FAILED_STAGE)
}

/** Progress of the long-running operations the kernel is reporting. */
export interface ProgressState {
  byDoc: Record<DocId, Progress>
  /** Subscribes to kernel progress events; returns an unsubscribe function. */
  start: () => () => void
  /** Shows a line at once, before the first event arrives. */
  begin: (doc: DocId) => void
  /** Removes the line for one document. */
  clear: (doc: DocId) => void
}

/** Creates the initial, action-free part of the progress state. */
export function initialProgressState(): Pick<ProgressState, 'byDoc'> {
  return { byDoc: {} }
}

export const useProgressStore = create<ProgressState>((set) => {
  const clear = (doc: DocId): void => {
    set((state) => {
      if (!(doc in state.byDoc)) return state
      const byDoc = { ...state.byDoc }
      delete byDoc[doc]
      return { byDoc }
    })
  }

  return {
    ...initialProgressState(),

    start: () =>
      onProgressEvent((event) => {
        if (isTerminal(event.stage, event.percent)) {
          clear(event.doc)
          // The terminal failure carries the only message the user gets when
          // the operation broke outside the request that started it.
          if (event.stage === FAILED_STAGE && event.message)
            useUiStore
              .getState()
              .pushToast({ kind: 'error', message: event.message })
          return
        }
        set((state) => ({
          byDoc: {
            ...state.byDoc,
            [event.doc]: { stage: event.stage, percent: event.percent },
          },
        }))
      }),

    begin: (doc) =>
      set((state) => ({
        byDoc: { ...state.byDoc, [doc]: { stage: COMPILE_STAGE, percent: 0 } },
      })),

    clear,
  }
})

/**
 * Selector factory: the progress line to draw. The active document's operation
 * wins; any other running operation is shown when the active document has none,
 * so switching tabs mid-export does not hide the feedback.
 */
export function selectProgress(doc: DocId | null) {
  return (state: ProgressState): Progress | null => {
    if (doc !== null && state.byDoc[doc]) return state.byDoc[doc]
    return Object.values(state.byDoc)[0] ?? null
  }
}
