import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

const desktop = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  onCloseRequested: vi.fn(),
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: desktop.invoke }))
vi.mock('@tauri-apps/api/event', () => ({ listen: desktop.listen }))
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ onCloseRequested: desktop.onCloseRequested }),
}))

import {
  installWindowCloseGuard,
  onWindowClosed,
  onWindowRejoin,
  readWindowTab,
  readWindowTarget,
  rejoinWindow,
  requestWindow,
  writeWindowTab,
  type WindowTarget,
} from './windows'
import { windowTargetLabel, windowTargetUrl } from './windowTargets'

const target: WindowTarget = {
  kind: 'tab',
  doc: 7,
  tab: 'glyph',
  gid: 4,
  tabId: 'glyph:7:4:2',
}

beforeEach(() => {
  localStorage.clear()
  desktop.invoke.mockReset().mockResolvedValue(undefined)
  desktop.listen.mockReset().mockResolvedValue(vi.fn())
  desktop.onCloseRequested.mockReset().mockResolvedValue(vi.fn())
})
afterEach(() => {
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
  window.history.replaceState({}, '', '/')
})

describe('workspace window targets', () => {
  it.each<WindowTarget>([
    { kind: 'document', doc: 1 },
    target,
    { kind: 'panel', doc: 2, panel: 'inspector' },
    { kind: 'panel', doc: 2, panel: 'preview' },
  ])('round trips $kind targets', (value) => {
    const url = new URL(windowTargetUrl(value), window.location.href)
    expect(readWindowTarget(url.search)).toEqual(value)
  })

  it.each([
    '',
    '?fcWindow=alien&doc=1',
    '?fcWindow=document&doc=-1',
    '?fcWindow=document&doc=1.5',
    '?fcWindow=document&doc=4294967296',
    '?fcWindow=tab&doc=1&tab=alien',
    '?fcWindow=panel&doc=1&panel=alien',
    '?fcWindow=tab&doc=1&tab=glyph&gid=oops',
    '?fcWindow=tab&doc=1&tab=glyph&tabId=%3Cscript%3E',
  ])('ignores malformed routes %s', (search) => {
    expect(readWindowTarget(search)).toBeNull()
  })

  it('distinguishes two text tabs for the same glyph', () => {
    expect(windowTargetLabel(target)).not.toEqual(
      windowTargetLabel({ ...target, tabId: 'glyph:7:4:3' }),
    )
  })

  it('keeps a tab identity and its latest glyph when its edited text changes', () => {
    const tab = { id: 'glyph:7:4:2', kind: 'glyph' as const, doc: 7, gid: 8 }
    writeWindowTab(target, tab)
    expect(readWindowTab(target)).toEqual(tab)
    expect(windowTargetLabel(target)).toEqual(
      windowTargetLabel({ ...target, gid: 8 }),
    )
  })

  it('hands off full text tab state without embedding it in URLs', async () => {
    vi.stubGlobal('__TAURI_INTERNALS__', {})
    const tab = {
      id: 'glyph:7:4:2',
      kind: 'glyph' as const,
      doc: 7,
      gid: 4,
      textState: {
        text: 'AV text',
        activeIndex: 1,
        mode: 'text' as const,
        master: 'm1',
        size: 72,
        features: [['liga', false] as [string, boolean]],
        direction: 'rtl' as const,
        previewTheme: 'dark' as const,
        previewCollapsed: true,
      },
    }
    await requestWindow(target, tab)
    expect(readWindowTab(target)).toEqual(tab)
    expect(windowTargetUrl(target)).not.toContain('AV')
    expect(desktop.invoke).toHaveBeenCalledWith('open_workspace_window', {
      target,
    })
  })

  it('does not reuse a mismatched tab payload', async () => {
    vi.stubGlobal('__TAURI_INTERNALS__', {})
    await expect(
      requestWindow(target, {
        id: 'other',
        kind: 'glyph',
        doc: 8,
        gid: 4,
      }),
    ).rejects.toThrow()
    expect(desktop.invoke).not.toHaveBeenCalled()
  })
})

describe('browser window lifecycle', () => {
  it('focuses the same open popup without reloading its editor', async () => {
    const popup = { closed: false, focus: vi.fn() }
    const open = vi
      .spyOn(window, 'open')
      .mockReturnValue(popup as unknown as Window)
    const value: WindowTarget = { kind: 'document', doc: 81 }
    await requestWindow(value)
    await requestWindow(value)
    expect(open).toHaveBeenCalledOnce()
    expect(popup.focus).toHaveBeenCalledTimes(2)
    popup.closed = true
    await requestWindow(value)
    expect(open).toHaveBeenCalledTimes(2)
  })

  it('reports popup blocking instead of silently detaching', async () => {
    vi.spyOn(window, 'open').mockReturnValue(null)
    await expect(requestWindow({ kind: 'document', doc: 82 })).rejects.toThrow()
  })

  it('warns only for dirty browser closes and disposes listeners', () => {
    let dirty = true
    const stop = installWindowCloseGuard({
      isDirty: () => dirty,
      confirmClose: vi.fn(),
    })
    const request = () => {
      const event = new Event('beforeunload', { cancelable: true })
      window.dispatchEvent(event)
      return event.defaultPrevented
    }
    expect(request()).toBe(true)
    dirty = false
    expect(request()).toBe(false)
    dirty = true
    stop()
    expect(request()).toBe(false)
  })

  it('notifies rejoin receivers through cross-window storage and rejects malformed data', () => {
    const listener = vi.fn()
    const stop = onWindowRejoin(listener)
    window.dispatchEvent(
      new StorageEvent('storage', {
        key: 'fontcarver:window-event',
        newValue: JSON.stringify({ kind: 'rejoin', target, nonce: 1 }),
      }),
    )
    window.dispatchEvent(
      new StorageEvent('storage', {
        key: 'fontcarver:window-event',
        newValue: '{invalid',
      }),
    )
    expect(listener).toHaveBeenCalledExactlyOnceWith(target)
    stop()
  })

  it('signals closed detached windows at pagehide, after cancellation is impossible', () => {
    window.history.replaceState({}, '', windowTargetUrl(target))
    const set = vi.spyOn(Storage.prototype, 'setItem')
    const stop = installWindowCloseGuard({
      isDirty: () => true,
      confirmClose: vi.fn(),
    })
    window.dispatchEvent(new Event('beforeunload', { cancelable: true }))
    expect(set).not.toHaveBeenCalled()
    window.dispatchEvent(new PageTransitionEvent('pagehide'))
    expect(set).toHaveBeenCalledWith(
      'fontcarver:window-event',
      expect.stringContaining('"kind":"closed"'),
    )
    stop()
  })
})

describe('native window lifecycle', () => {
  beforeEach(() => vi.stubGlobal('__TAURI_INTERNALS__', {}))

  it('routes rejoin through the shell without closing a kernel document', async () => {
    await rejoinWindow(target)
    expect(desktop.invoke).toHaveBeenCalledExactlyOnceWith(
      'rejoin_workspace_window',
      { target },
    )
  })

  it('subscribes to validated native close/rejoin events and cleans up', async () => {
    const unlisten = vi.fn()
    desktop.listen.mockResolvedValue(unlisten)
    const receive = vi.fn()
    const stop = onWindowClosed(receive)
    await Promise.resolve()
    const callback = desktop.listen.mock.calls[0][1]
    callback({ payload: target })
    callback({ payload: { ...target, doc: -1 } })
    expect(receive).toHaveBeenCalledExactlyOnceWith(target)
    stop()
    expect(unlisten).toHaveBeenCalledOnce()
  })

  it('keeps cancellation open and closes only after async confirmation', async () => {
    const confirmClose = vi
      .fn()
      .mockResolvedValueOnce(false)
      .mockResolvedValueOnce(true)
    const stop = installWindowCloseGuard({ isDirty: () => true, confirmClose })
    await Promise.resolve()
    const handler = desktop.onCloseRequested.mock.calls[0][0]
    const event = { preventDefault: vi.fn() }
    await handler(event)
    expect(desktop.invoke).not.toHaveBeenCalled()
    await handler(event)
    expect(event.preventDefault).toHaveBeenCalledTimes(2)
    expect(desktop.invoke).toHaveBeenCalledExactlyOnceWith(
      'close_workspace_window',
    )
    stop()
  })

  it('coalesces repeated OS close clicks and reports confirmation errors', async () => {
    let reject!: (error: unknown) => void
    const confirmClose = vi.fn(
      () =>
        new Promise<boolean>((_resolve, fail) => {
          reject = fail
        }),
    )
    const onError = vi.fn()
    const stop = installWindowCloseGuard({
      isDirty: () => true,
      confirmClose,
      onError,
    })
    await Promise.resolve()
    const handler = desktop.onCloseRequested.mock.calls[0][0]
    const event = { preventDefault: vi.fn() }
    const first = handler(event)
    await handler(event)
    expect(confirmClose).toHaveBeenCalledOnce()
    const error = new Error('save failed')
    reject(error)
    await first
    expect(onError).toHaveBeenCalledWith(error)
    expect(desktop.invoke).not.toHaveBeenCalled()
    stop()
  })

  it('disposes a native listener even when registration finishes after unmount', async () => {
    const unlisten = vi.fn()
    const stop = installWindowCloseGuard({
      isDirty: () => false,
      confirmClose: vi.fn(),
    })
    stop()
    await Promise.resolve()
    expect(desktop.onCloseRequested).toHaveBeenCalledOnce()
    desktop.onCloseRequested.mockReset().mockResolvedValue(unlisten)
    const stopAgain = installWindowCloseGuard({
      isDirty: () => false,
      confirmClose: vi.fn(),
    })
    stopAgain()
    await Promise.resolve()
    expect(unlisten).toHaveBeenCalledOnce()
  })
})
