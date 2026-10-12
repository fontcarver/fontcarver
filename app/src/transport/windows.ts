import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'

import type { Tab } from '../stores/uiStore'
import {
  isWindowTarget,
  readWindowTarget,
  windowTargetLabel,
  windowTargetUrl,
  writeWindowTab,
  type WindowTarget,
} from './windowTargets'

export {
  readWindowTarget,
  readWindowTab,
  writeWindowTab,
  type WindowTarget,
} from './windowTargets'

const openedWindows = new Map<string, Window>()
const eventKey = 'fontcarver:window-event'
type WindowEventKind = 'rejoin' | 'closed'

/** Whether this frontend is hosted by the native desktop shell. */
export function isDesktopWindow(): boolean {
  return '__TAURI_INTERNALS__' in window
}

/** Opens or focuses a view; every view continues to use the shared kernel session. */
export async function requestWindow(
  target: WindowTarget,
  tab?: Tab,
): Promise<void> {
  const label = windowTargetLabel(target)
  if (tab) writeWindowTab(target, tab)
  if (isDesktopWindow()) {
    await invoke('open_workspace_window', { target })
    return
  }
  let popup = openedWindows.get(label)
  if (!popup || popup.closed) {
    popup =
      window.open(
        windowTargetUrl(target),
        label,
        'popup,width=1200,height=850',
      ) ?? undefined
    if (!popup) throw new Error('The browser blocked the workspace window')
    openedWindows.set(label, popup)
  }
  popup.focus()
}

function emitBrowserEvent(kind: WindowEventKind, target: WindowTarget): void {
  localStorage.setItem(
    eventKey,
    JSON.stringify({ kind, target, nonce: crypto.randomUUID() }),
  )
}

function subscribe(
  kind: WindowEventKind,
  callback: (target: WindowTarget) => void,
): () => void {
  if (isDesktopWindow()) {
    let active = true
    let unlisten: UnlistenFn | undefined
    void listen<unknown>(`fc:window-${kind}`, ({ payload }) => {
      if (active && isWindowTarget(payload)) callback(payload)
    }).then(
      (stop) => {
        if (active) unlisten = stop
        else stop()
      },
      () => undefined,
    )
    return () => {
      active = false
      unlisten?.()
    }
  }
  const receive = (event: StorageEvent) => {
    if (event.key !== eventKey || !event.newValue) return
    try {
      const message: unknown = JSON.parse(event.newValue)
      if (
        message &&
        typeof message === 'object' &&
        'kind' in message &&
        message.kind === kind &&
        'target' in message &&
        isWindowTarget(message.target)
      )
        callback(message.target)
    } catch {
      /* Ignore unrelated or malformed storage signals. */
    }
  }
  window.addEventListener('storage', receive)
  return () => window.removeEventListener('storage', receive)
}

/** Receives requests to restore a detached view to its owning workspace. */
export function onWindowRejoin(
  callback: (target: WindowTarget) => void,
): () => void {
  return subscribe('rejoin', callback)
}

/** Receives completed closes so the owner can restore detached views. */
export function onWindowClosed(
  callback: (target: WindowTarget) => void,
): () => void {
  return subscribe('closed', callback)
}

/** Closes this view only; document lifetime belongs to the shell's close workflow. */
export async function closeCurrentWindow(): Promise<void> {
  if (isDesktopWindow()) await invoke('close_workspace_window')
  else window.close()
}

/** Restores a detached view in the main workspace before closing its window. */
export async function rejoinWindow(
  target: WindowTarget,
  tab?: Tab,
): Promise<void> {
  if (tab) writeWindowTab(target, tab)
  if (isDesktopWindow()) {
    await invoke('rejoin_workspace_window', { target })
    return
  }
  if (!window.opener || (window.opener as Window).closed) {
    throw new Error('The owning workspace window is no longer open')
  }
  emitBrowserEvent('rejoin', target)
  ;(window.opener as Window).focus()
  window.close()
}

/** Delegates native close decisions to the application and protects browser exits. */
export interface WindowCloseGuard {
  /** Synchronous browser dirty check; browsers own the final confirmation text. */
  isDirty: () => boolean
  /** Settles edits and returns true only when closing the native view is allowed. */
  confirmClose: () => Promise<boolean>
  /** Receives failed native close decisions or event registrations. */
  onError?: (error: unknown) => void
}

/** Installs one close guard and returns cleanup safe during asynchronous registration. */
export function installWindowCloseGuard(guard: WindowCloseGuard): () => void {
  let active = true
  if (!isDesktopWindow()) {
    const beforeUnload = (event: BeforeUnloadEvent) => {
      if (!guard.isDirty()) return
      event.preventDefault()
      event.returnValue = ''
    }
    const pageHide = (event: PageTransitionEvent) => {
      const target = readWindowTarget()
      if (target && !event.persisted) {
        try {
          emitBrowserEvent('closed', target)
        } catch (error) {
          guard.onError?.(error)
        }
      }
    }
    window.addEventListener('beforeunload', beforeUnload)
    window.addEventListener('pagehide', pageHide)
    return () => {
      window.removeEventListener('beforeunload', beforeUnload)
      window.removeEventListener('pagehide', pageHide)
    }
  }
  let unlisten: UnlistenFn | undefined
  let closing = false
  void getCurrentWindow()
    .onCloseRequested(async (event) => {
      event.preventDefault()
      if (!active || closing) return
      closing = true
      try {
        if ((await guard.confirmClose()) && active) await closeCurrentWindow()
      } catch (error) {
        guard.onError?.(error)
      } finally {
        closing = false
      }
    })
    .then(
      (stop) => {
        if (active) unlisten = stop
        else stop()
      },
      (error: unknown) => guard.onError?.(error),
    )
  return () => {
    active = false
    unlisten?.()
  }
}
