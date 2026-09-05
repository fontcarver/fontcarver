import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { setTransport, type Prefs } from '../api'
import { createMockTransport } from '../test/mockTransport'

import { DEFAULT_PREFS, initialPrefsState, usePrefsStore } from './prefsStore'
import { initialUiState, useUiStore } from './uiStore'

function stored(overrides: Partial<Prefs> = {}): Prefs {
  return { ...DEFAULT_PREFS, ...overrides }
}

beforeEach(() => {
  localStorage.clear()
  useUiStore.setState(initialUiState())
  usePrefsStore.setState(initialPrefsState())
})

afterEach(() => {
  setTransport(undefined)
})

describe('prefsStore', () => {
  it('loads preferences and mirrors theme and language into the shell', async () => {
    const transport = createMockTransport({
      'prefs.get': () => stored({ theme: 'dark', language: 'zh-CN' }),
    })
    setTransport(transport)

    await usePrefsStore.getState().load()

    expect(usePrefsStore.getState().prefs.theme).toBe('dark')
    expect(usePrefsStore.getState().loaded).toBe(true)
    expect(useUiStore.getState().theme).toBe('dark')
    expect(useUiStore.getState().language).toBe('zh-CN')
  })

  it('sends the whole preference record on every change', async () => {
    const transport = createMockTransport({
      'prefs.get': () => stored(),
      'prefs.set': ({ prefs }) => prefs,
    })
    setTransport(transport)
    await usePrefsStore.getState().load()

    await usePrefsStore.getState().update({ grid_size: 8 })

    const last = transport.calls.at(-1)
    expect(last?.method).toBe('prefs.set')
    expect(last?.params).toEqual({ prefs: stored({ grid_size: 8 }) })
    expect(usePrefsStore.getState().prefs.grid_size).toBe(8)
  })

  it('round trips a change through the kernel', async () => {
    let saved: Prefs = stored()
    const transport = createMockTransport({
      'prefs.get': () => saved,
      'prefs.set': ({ prefs }) => {
        saved = prefs
        return prefs
      },
    })
    setTransport(transport)
    await usePrefsStore.getState().load()

    await usePrefsStore.getState().update({
      nudge: [2, 20, 200],
      snap: { ...DEFAULT_PREFS.snap, grid: false },
    })
    usePrefsStore.setState(initialPrefsState())
    await usePrefsStore.getState().load()

    expect(usePrefsStore.getState().prefs.nudge).toEqual([2, 20, 200])
    expect(usePrefsStore.getState().prefs.snap.grid).toBe(false)
  })

  it('restores the shipped defaults', async () => {
    const transport = createMockTransport({
      'prefs.get': () => stored({ autosave_seconds: 5, grid_size: 40 }),
      'prefs.set': ({ prefs }) => prefs,
    })
    setTransport(transport)
    await usePrefsStore.getState().load()

    await usePrefsStore.getState().reset()

    expect(usePrefsStore.getState().prefs).toEqual(DEFAULT_PREFS)
    expect(transport.calls.at(-1)?.params).toEqual({ prefs: DEFAULT_PREFS })
  })

  it('keeps the last good values and reports a failed write', async () => {
    const transport = createMockTransport({
      'prefs.get': () => stored(),
      'prefs.set': () => {
        throw new Error('config directory is read-only')
      },
    })
    setTransport(transport)
    await usePrefsStore.getState().load()

    await usePrefsStore.getState().update({ grid_size: 12 })

    expect(usePrefsStore.getState().prefs.grid_size).toBe(
      DEFAULT_PREFS.grid_size,
    )
    expect(useUiStore.getState().toasts.at(-1)?.kind).toBe('error')
  })
})
