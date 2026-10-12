import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { setTransport } from '../api'
import { createMockTransport, makeSummary } from '../test/mockTransport'

import {
  initialSessionState,
  isNativeDocumentPath,
  useSessionStore,
} from './sessionStore'
import { initialUiState, useUiStore } from './uiStore'

beforeEach(() => {
  useSessionStore.setState(initialSessionState())
  useUiStore.setState(initialUiState())
})

afterEach(() => {
  setTransport(undefined)
  vi.restoreAllMocks()
})

describe('directory project persistence', () => {
  it.each(['/fonts/Family.fcfp', 'C:\\Fonts\\Family.FCFP'])(
    'recognizes %s as a native save target',
    (path) => {
      expect(isNativeDocumentPath(path)).toBe(true)
    },
  )

  it.each([null, '/fonts/Family.fcfp.backup', '/fonts/Family.ttf'])(
    'does not treat %s as a native save target',
    (path) => {
      expect(isNativeDocumentPath(path)).toBe(false)
    },
  )

  it('saves an open directory project without asking for another path', async () => {
    const saved = makeSummary({ path: '/fonts/Family.fcfp', dirty: false })
    const transport = createMockTransport({
      'session.save': () => saved,
      'session.recent': () => ['/fonts/Family.fcfp'],
    })
    setTransport(transport)
    useSessionStore.setState({ docs: [{ ...saved, dirty: true }] })

    expect(await useSessionStore.getState().save(saved.doc)).toBe(true)

    expect(transport.calls[0]).toEqual({
      method: 'session.save',
      params: { doc: saved.doc, path: null },
    })
    expect(useUiStore.getState().pathPrompt).toBeNull()
    expect(useSessionStore.getState().docs).toEqual([saved])
  })

  it('saves a directory project before closing its document', async () => {
    const saved = makeSummary({ path: '/fonts/Family.fcfp', dirty: false })
    const transport = createMockTransport({
      'session.save': () => saved,
      'session.recent': () => [],
      'doc.summary': () => saved,
      'session.close': () => null,
    })
    setTransport(transport)
    useSessionStore.setState({ docs: [{ ...saved, dirty: true }] })

    await useSessionStore.getState().saveAndClose(saved.doc)

    const methods = transport.calls.map((entry) => entry.method)
    expect(methods.indexOf('session.save')).toBe(0)
    expect(methods.indexOf('session.close')).toBeGreaterThan(0)
    expect(useSessionStore.getState().docs).toEqual([])
  })

  it('keeps a project open when its save fails', async () => {
    const summary = makeSummary({ path: '/fonts/Family.fcfp', dirty: true })
    const transport = createMockTransport({
      'session.save': () => {
        throw new Error('Project is read-only')
      },
    })
    setTransport(transport)
    useSessionStore.setState({ docs: [summary] })

    await useSessionStore.getState().saveAndClose(summary.doc)

    expect(transport.calls.map((entry) => entry.method)).toEqual([
      'session.save',
    ])
    expect(useSessionStore.getState().docs).toEqual([summary])
    expect(useUiStore.getState().toasts[0]?.message).toBe(
      'Project is read-only',
    )
  })

  it('uses the path prompt to open a project in browser mode', async () => {
    const transport = createMockTransport({})
    setTransport(transport)

    await useSessionStore.getState().openProjectDialog()

    expect(useUiStore.getState().pathPrompt).toEqual({ mode: 'open' })
    expect(transport.calls).toEqual([])
  })
})
