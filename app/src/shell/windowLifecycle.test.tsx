import { act, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { setTransport } from '../api'
import { registerDocumentEditParticipant } from '../editor/documentEditParticipants'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { initialUiState, useUiStore, type Tab } from '../stores/uiStore'
import { createMockTransport, makeSummary } from '../test/mockTransport'
import type { WindowTarget } from '../transport/windows'
import { CloseDocumentDialog } from './CloseDocumentDialog'
import { installWindowLifecycle } from './windowLifecycle'

const windows = vi.hoisted(() => ({
  target: null as WindowTarget | null,
  desktop: false,
  payload: null as Tab | null,
  guard: null as null | {
    isDirty: () => boolean
    confirmClose: () => Promise<boolean>
  },
  rejoin: null as null | ((target: WindowTarget) => void),
  closed: null as null | ((target: WindowTarget) => void),
  request: vi.fn<(target: WindowTarget, tab?: Tab) => Promise<void>>(),
  close: vi.fn<() => Promise<void>>(),
  write: vi.fn(),
}))

vi.mock('../transport/windows', () => ({
  readWindowTarget: () => windows.target,
  readWindowTab: () => windows.payload,
  isDesktopWindow: () => windows.desktop,
  requestWindow: windows.request,
  closeCurrentWindow: windows.close,
  writeWindowTab: windows.write,
  onWindowRejoin: (callback: (target: WindowTarget) => void) => {
    windows.rejoin = callback
    return () => {
      windows.rejoin = null
    }
  },
  onWindowClosed: (callback: (target: WindowTarget) => void) => {
    windows.closed = callback
    return () => {
      windows.closed = null
    }
  },
  installWindowCloseGuard: (guard: typeof windows.guard) => {
    windows.guard = guard
    return () => {
      windows.guard = null
    }
  },
}))

let stop = () => {}

beforeEach(() => {
  localStorage.clear()
  useUiStore.setState(initialUiState())
  useSessionStore.setState(initialSessionState())
  windows.target = null
  windows.desktop = false
  windows.payload = null
  windows.request.mockReset().mockResolvedValue()
  windows.close.mockReset().mockResolvedValue()
  windows.write.mockReset()
})

afterEach(() => {
  stop()
  setTransport(undefined)
})

function loaded(doc = 1) {
  useSessionStore.setState({ docs: [makeSummary({ doc })] })
  useUiStore.getState().openTab({ kind: 'font', doc })
}

describe('window lifecycle routing', () => {
  it('waits for the target document and restores its Overview plus transferred tab', () => {
    windows.target = {
      kind: 'tab',
      doc: 2,
      tab: 'glyph',
      gid: 7,
      tabId: 'glyph:2:4',
    }
    windows.payload = {
      id: 'glyph:2:4',
      kind: 'glyph',
      doc: 2,
      gid: 7,
      label: 'Proof',
      textState: {
        text: 'Hamburgefonts',
        activeIndex: 2,
        mode: 'text',
        master: null,
        size: 72,
        features: [['kern', true]],
        direction: 'ltr',
        previewTheme: 'dark',
        previewCollapsed: false,
      },
    }
    stop = installWindowLifecycle()
    expect(useUiStore.getState().tabs).toEqual([])
    useSessionStore.setState({ docs: [makeSummary(), makeSummary({ doc: 2 })] })
    expect(useUiStore.getState().tabs).toEqual([
      { id: 'font:2', kind: 'font', doc: 2 },
      windows.payload,
    ])
    expect(useUiStore.getState().activeTab).toBe('glyph:2:4')
    expect(windows.request).not.toHaveBeenCalled()
  })

  it('routes the second desktop document into a dedicated window once', async () => {
    windows.desktop = true
    loaded()
    stop = installWindowLifecycle()
    useSessionStore.setState({ docs: [makeSummary(), makeSummary({ doc: 2 })] })
    useUiStore.getState().openTab({ kind: 'font', doc: 2 })
    await waitFor(() =>
      expect(useUiStore.getState().tabs.map((tab) => tab.doc)).toEqual([1]),
    )
    expect(windows.request).toHaveBeenCalledExactlyOnceWith(
      { kind: 'document', doc: 2 },
      { id: 'font:2', kind: 'font', doc: 2 },
    )
  })

  it('preserves the newly opened tab if the platform cannot open a window', async () => {
    windows.desktop = true
    windows.request.mockRejectedValueOnce(new Error('window blocked'))
    loaded()
    stop = installWindowLifecycle()
    useUiStore.getState().openTab({ kind: 'font', doc: 2 })
    await waitFor(() => expect(useUiStore.getState().toasts).toHaveLength(1))
    expect(useUiStore.getState().tabs.map((tab) => tab.doc)).toEqual([1, 2])
    expect(windows.request).toHaveBeenCalledOnce()
  })

  it('keeps multiple documents in the normal browser window', () => {
    loaded()
    stop = installWindowLifecycle()
    useUiStore.getState().openTab({ kind: 'font', doc: 2 })
    expect(useUiStore.getState().tabs.map((tab) => tab.doc)).toEqual([1, 2])
    expect(windows.request).not.toHaveBeenCalled()
  })

  it('restores a closed detached tab and panel only in their owning document window', async () => {
    windows.target = { kind: 'document', doc: 1 }
    loaded()
    stop = installWindowLifecycle()
    windows.payload = { id: 'features:1', kind: 'features', doc: 1 }
    windows.rejoin?.({ kind: 'tab', doc: 1, tab: 'features' })
    await waitFor(() =>
      expect(useUiStore.getState().activeTab).toBe('features:1'),
    )
    useUiStore.getState().setPanelVisible('inspector', false)
    windows.closed?.({ kind: 'panel', doc: 1, panel: 'inspector' })
    expect(useUiStore.getState().panels.inspector).toBe(true)
    windows.payload = { id: 'fontinfo:2', kind: 'fontinfo', doc: 2 }
    windows.rejoin?.({ kind: 'tab', doc: 2, tab: 'fontinfo' })
    expect(useUiStore.getState().tabs.every((tab) => tab.doc === 1)).toBe(true)
  })

  it('refreshes on focus and closes a target whose document was closed in another window', async () => {
    windows.target = { kind: 'document', doc: 1 }
    loaded()
    setTransport(createMockTransport({ 'session.list': () => [] }))
    stop = installWindowLifecycle()
    window.dispatchEvent(new Event('focus'))
    await waitFor(() => expect(windows.close).toHaveBeenCalledOnce())
    expect(useSessionStore.getState().docs).toEqual([])
    expect(useUiStore.getState().tabs).toEqual([])
  })

  it('finishes the current draft before activating a rejoined tab', async () => {
    loaded()
    let finish!: () => void
    const pending = new Promise<void>((resolve) => {
      finish = resolve
    })
    const dispose = registerDocumentEditParticipant(1, { flush: () => pending })
    stop = installWindowLifecycle()
    windows.payload = { id: 'features:1', kind: 'features', doc: 1 }
    try {
      windows.rejoin?.({ kind: 'tab', doc: 1, tab: 'features' })
      expect(useUiStore.getState().activeTab).toBe('font:1')
      expect(
        useUiStore.getState().tabs.some((tab) => tab.id === 'features:1'),
      ).toBe(true)
      finish()
      await waitFor(() =>
        expect(useUiStore.getState().activeTab).toBe('features:1'),
      )
    } finally {
      finish()
      dispose()
    }
  })

  it('reports storage failure without throwing from a UI subscription', () => {
    windows.target = { kind: 'tab', doc: 1, tab: 'features' }
    windows.payload = { id: 'features:1', kind: 'features', doc: 1 }
    loaded()
    stop = installWindowLifecycle()
    windows.write.mockImplementationOnce(() => {
      throw new Error('storage full')
    })
    expect(() =>
      useUiStore.getState().setTabLabel('features:1', 'Draft'),
    ).not.toThrow()
    expect(useUiStore.getState().toasts).toHaveLength(1)
  })

  it('does not overwrite documents opened or edited while the focus refresh was pending', async () => {
    loaded()
    let finish!: (docs: ReturnType<typeof makeSummary>[]) => void
    setTransport(
      createMockTransport({
        'session.list': () =>
          new Promise((resolve) => {
            finish = resolve
          }),
      }),
    )
    stop = installWindowLifecycle()
    window.dispatchEvent(new Event('focus'))
    const edited = makeSummary({ dirty: true })
    const opened = makeSummary({ doc: 2 })
    useSessionStore.setState({ docs: [edited, opened] })
    finish([makeSummary()])
    await waitFor(() =>
      expect(useSessionStore.getState().docs).toEqual([edited, opened]),
    )
    await Promise.resolve()
    expect(useSessionStore.getState().docs).toEqual([edited, opened])
  })
})

describe('window close decisions', () => {
  it('requests the current kernel summary before allowing a cached-clean document to close', async () => {
    windows.desktop = true
    windows.target = { kind: 'document', doc: 1 }
    loaded()
    const transport = createMockTransport({
      'doc.summary': () => makeSummary(),
      'session.close': () => null,
    })
    setTransport(transport)
    stop = installWindowLifecycle()
    expect(await windows.guard?.confirmClose()).toBe(true)
    expect(transport.calls.map((call) => call.method)).toEqual([
      'doc.summary',
      'session.close',
    ])
    // The native guard owns the actual close after a true decision.
    expect(windows.close).not.toHaveBeenCalled()
  })

  it('keeps the native document window open when saving fails', async () => {
    const user = userEvent.setup()
    windows.desktop = true
    windows.target = { kind: 'document', doc: 1 }
    loaded()
    const transport = createMockTransport({
      'doc.summary': () => makeSummary({ path: '/font.fcf', dirty: true }),
      'session.save': () => {
        throw new Error('disk full')
      },
    })
    setTransport(transport)
    stop = installWindowLifecycle()
    render(<CloseDocumentDialog />)
    await act(async () => {
      expect(await windows.guard?.confirmClose()).toBe(false)
    })
    await user.click(screen.getByRole('button', { name: 'Save' }))
    await waitFor(() => expect(useUiStore.getState().toasts).toHaveLength(1))
    expect(useSessionStore.getState().docs).toHaveLength(1)
    expect(windows.close).not.toHaveBeenCalled()
    expect(
      transport.calls.some((call) => call.method === 'session.close'),
    ).toBe(false)
  })

  it('flushes a draft on cached-clean documents, preserves Cancel, and saves before closing the window', async () => {
    const user = userEvent.setup()
    windows.desktop = true
    windows.target = { kind: 'document', doc: 1 }
    loaded()
    let dirty = false
    let draft = true
    const dispose = registerDocumentEditParticipant(1, {
      hasPendingEdits: () => draft,
      flush: async () => {
        if (draft) {
          dirty = true
          draft = false
        }
      },
    })
    const transport = createMockTransport({
      'doc.summary': () => makeSummary({ path: '/font.fcf', dirty }),
      'session.save': () => {
        dirty = false
        return makeSummary({ path: '/font.fcf', dirty })
      },
      'session.close': () => null,
      'session.recent': () => [],
    })
    setTransport(transport)
    stop = installWindowLifecycle()
    render(<CloseDocumentDialog />)
    try {
      expect(windows.guard?.isDirty()).toBe(true)
      await act(async () => {
        expect(await windows.guard?.confirmClose()).toBe(false)
      })
      await user.click(screen.getByRole('button', { name: 'Cancel' }))
      expect(useSessionStore.getState().docs).toHaveLength(1)
      expect(windows.close).not.toHaveBeenCalled()
      await act(async () => {
        expect(await windows.guard?.confirmClose()).toBe(false)
      })
      await user.click(screen.getByRole('button', { name: 'Save' }))
      await waitFor(() => expect(windows.close).toHaveBeenCalledOnce())
      expect(transport.calls.map((call) => call.method)).toEqual([
        'doc.summary',
        'doc.summary',
        'session.save',
        'session.recent',
        'doc.summary',
        'session.close',
      ])
    } finally {
      dispose()
    }
  })

  it('settles a detached tab without closing the shared document', async () => {
    windows.desktop = true
    windows.target = { kind: 'tab', doc: 1, tab: 'features' }
    windows.payload = { id: 'features:1', kind: 'features', doc: 1 }
    loaded()
    let flushed = false
    const dispose = registerDocumentEditParticipant(1, {
      flush: async () => {
        flushed = true
      },
    })
    const transport = createMockTransport()
    setTransport(transport)
    stop = installWindowLifecycle()
    try {
      expect(await windows.guard?.confirmClose()).toBe(true)
      expect(flushed).toBe(true)
      expect(useSessionStore.getState().docs).toHaveLength(1)
      expect(transport.calls).toEqual([])
      expect(windows.write).toHaveBeenLastCalledWith(
        windows.target,
        windows.payload,
      )
    } finally {
      dispose()
    }
  })

  it('keeps a detached window open when a draft cannot be persisted', async () => {
    windows.target = { kind: 'panel', doc: 1, panel: 'inspector' }
    loaded()
    const dispose = registerDocumentEditParticipant(1, {
      flush: async () => {
        throw new Error('save failed')
      },
    })
    stop = installWindowLifecycle()
    try {
      await expect(windows.guard?.confirmClose()).rejects.toThrow('save failed')
      expect(windows.close).not.toHaveBeenCalled()
    } finally {
      dispose()
    }
  })
})
