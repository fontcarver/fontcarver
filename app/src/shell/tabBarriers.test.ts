import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import * as editors from '../editor/activeEditor'
import { setTransport } from '../api'
import type { EditorController } from '../editor/editorController'
import { holdDocumentEdits } from '../editor/editSession'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { setCommandErrorReporter } from './commands'
import { createEditingTransport } from '../test/mockTransport'
import { KerningEditRunner } from '../views/text/kerningEdits'
import {
  closeTab,
  openFontInfoTab,
  openGlyphTab,
  selectTab,
} from './tabActions'

function deferred() {
  let resolve!: () => void
  let reject!: (error: Error) => void
  const promise = new Promise<void>((yes, no) => {
    resolve = yes
    reject = no
  })
  return { promise, resolve, reject }
}

function tabs() {
  const old = useUiStore.getState().openTab({ kind: 'glyph', doc: 1, gid: 9 })
  const next = useUiStore.getState().openTab({ kind: 'font', doc: 2 })
  useUiStore.getState().setActiveTab(old)
  return { old, next }
}

function editor(prepareTransition: () => Promise<void>, documentId = 1) {
  return { prepareTransition, documentId } as unknown as EditorController
}

beforeEach(() => {
  localStorage.clear()
  useUiStore.setState(initialUiState())
})

afterEach(() => {
  setTransport(undefined)
  setCommandErrorReporter(() => undefined)
  vi.restoreAllMocks()
})

describe('tab edit handover', () => {
  it('also drains a new kerning run started while an earlier edit was closing', async () => {
    const { old, next } = tabs()
    const initial = deferred()
    const applying = deferred()
    const transport = createEditingTransport({
      hold: { on: 'doc.edit_apply', until: applying.promise },
    })
    setTransport(transport)
    const run = new KerningEditRunner({
      doc: 1,
      master: 'm1',
      left: { glyph: 'A' },
      right: { glyph: 'V' },
      label: 'Kerning',
      refreshStart: async () => -80,
      onEnd: vi.fn(),
      onError: vi.fn(),
    })
    const prepare = vi
      .fn(() => run.end())
      .mockImplementationOnce(() => initial.promise)
    vi.spyOn(editors, 'activeEditor').mockReturnValue(editor(prepare))
    const switched = selectTab(next)
    run.press(-1)
    initial.resolve()
    await vi.waitFor(() => expect(prepare).toHaveBeenCalledTimes(2))
    await vi.waitFor(() =>
      expect(
        transport.calls.some((entry) => entry.method === 'doc.edit_apply'),
      ).toBe(true),
    )
    expect(useUiStore.getState().activeTab).toBe(old)
    applying.resolve()
    expect(await switched).toBe(true)
    expect(transport.calls.at(-1)?.method).toBe('doc.edit_end')
    expect(useUiStore.getState().activeTab).toBe(next)
  })

  it('captures the old editor and waits for its edit and older document work', async () => {
    const { old, next } = tabs()
    const closing = deferred()
    const prior = deferred()
    const prepare = vi.fn(() => closing.promise)
    vi.spyOn(editors, 'activeEditor').mockReturnValue(editor(prepare))
    void holdDocumentEdits(1, prior.promise)

    const switched = selectTab(next)
    expect(prepare).toHaveBeenCalledOnce()
    expect(useUiStore.getState().activeTab).toBe(old)
    closing.resolve()
    await Promise.resolve()
    expect(useUiStore.getState().activeTab).toBe(old)
    prior.resolve()
    expect(await switched).toBe(true)
    expect(useUiStore.getState().activeTab).toBe(next)
  })

  it('keeps only the latest requested tab when two selections wait together', async () => {
    const { old, next } = tabs()
    const third = useUiStore.getState().openTab({ kind: 'font', doc: 3 })
    useUiStore.getState().setActiveTab(old)
    const closing = deferred()
    vi.spyOn(editors, 'activeEditor').mockReturnValue(
      editor(() => closing.promise),
    )
    const first = selectTab(next)
    const second = selectTab(third)
    closing.resolve()
    expect(await first).toBe(false)
    expect(await second).toBe(true)
    expect(useUiStore.getState().activeTab).toBe(third)
  })

  it('cancels delayed selection if the editor changed under the same tab', async () => {
    const { old, next } = tabs()
    const closing = deferred()
    const current = vi
      .spyOn(editors, 'activeEditor')
      .mockReturnValue(editor(() => closing.promise))
    const switched = selectTab(next)
    current.mockReturnValue(editor(async () => undefined))
    closing.resolve()
    expect(await switched).toBe(false)
    expect(useUiStore.getState().activeTab).toBe(old)
  })

  it('cancels delayed selection if another caller already changed tabs', async () => {
    const { old, next } = tabs()
    const closing = deferred()
    vi.spyOn(editors, 'activeEditor').mockReturnValue(
      editor(() => closing.promise),
    )
    const switched = selectTab(next)
    const external = useUiStore.getState().openTab({ kind: 'font', doc: 3 })
    closing.resolve()
    expect(await switched).toBe(false)
    expect(useUiStore.getState().activeTab).toBe(external)
    expect(useUiStore.getState().tabs.some((tab) => tab.id === old)).toBe(true)
  })

  it('reports a failed final edit and leaves the old tab active', async () => {
    const { old, next } = tabs()
    const closing = deferred()
    const report = vi.fn()
    setCommandErrorReporter(report)
    vi.spyOn(editors, 'activeEditor').mockReturnValue(
      editor(() => closing.promise),
    )
    const switched = selectTab(next)
    const error = new Error('last kerning edit failed')
    closing.reject(error)
    expect(await switched).toBe(false)
    expect(report).toHaveBeenCalledExactlyOnceWith(error)
    expect(useUiStore.getState().activeTab).toBe(old)
  })

  it('drains before removing the active text tab', async () => {
    const { old, next } = tabs()
    const closing = deferred()
    vi.spyOn(editors, 'activeEditor').mockReturnValue(
      editor(() => closing.promise),
    )
    const closed = closeTab(old)
    expect(useUiStore.getState().activeTab).toBe(old)
    expect(useUiStore.getState().tabs.some((tab) => tab.id === old)).toBe(true)
    closing.resolve()
    await closed
    expect(useUiStore.getState().activeTab).toBe(next)
    expect(useUiStore.getState().tabs.some((tab) => tab.id === old)).toBe(false)
  })

  it.each(['glyph', 'fontinfo'] as const)(
    'drains before opening a %s tab',
    async (kind) => {
      const { old } = tabs()
      const closing = deferred()
      vi.spyOn(editors, 'activeEditor').mockReturnValue(
        editor(() => closing.promise),
      )
      const opened = kind === 'glyph' ? openGlyphTab(1, 20) : openFontInfoTab(1)
      expect(useUiStore.getState().activeTab).toBe(old)
      expect(useUiStore.getState().tabs).toHaveLength(2)
      closing.resolve()
      const id = await opened
      expect(id).toBeDefined()
      expect(useUiStore.getState().activeTab).toBe(id)
      expect(useUiStore.getState().tabs).toHaveLength(3)
    },
  )
})
