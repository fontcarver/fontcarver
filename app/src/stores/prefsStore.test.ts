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

  it('reads the stored record before its first write', async () => {
    const saved = stored({ recent_limit: 5, autosave_seconds: 120 })
    const transport = createMockTransport({
      'prefs.get': () => saved,
      'prefs.set': ({ prefs }) => prefs,
    })
    setTransport(transport)

    // The View menu can change the theme before any dialog has loaded prefs.
    await usePrefsStore.getState().update({ theme: 'dark' })

    expect(transport.calls.map((entry) => entry.method)).toEqual([
      'prefs.get',
      'prefs.set',
    ])
    expect(transport.calls[1]?.params).toEqual({
      prefs: stored({ recent_limit: 5, autosave_seconds: 120, theme: 'dark' }),
    })
  })

  it('coalesces updates that race one unresolved first read', async () => {
    const saved = stored({ recent_limit: 5, autosave_seconds: 120 })
    let release = () => undefined as void
    const read = new Promise<void>((resolve) => {
      release = () => resolve()
    })
    const transport = createMockTransport({
      'prefs.get': async () => {
        await read
        return saved
      },
      'prefs.set': ({ prefs }) => prefs,
    })
    setTransport(transport)

    // A View-menu theme click and a dialog field, both before the start-up
    // read has come back.
    const first = usePrefsStore.getState().update({ theme: 'dark' })
    const second = usePrefsStore.getState().update({ grid_size: 8 })
    release()
    await Promise.all([first, second])

    // One read, one write, and neither patch is lost.
    expect(transport.calls.map((entry) => entry.method)).toEqual([
      'prefs.get',
      'prefs.set',
    ])
    expect(transport.calls[1]?.params).toEqual({
      prefs: stored({
        recent_limit: 5,
        autosave_seconds: 120,
        theme: 'dark',
        grid_size: 8,
      }),
    })
    expect(usePrefsStore.getState().prefs).toEqual(
      stored({
        recent_limit: 5,
        autosave_seconds: 120,
        theme: 'dark',
        grid_size: 8,
      }),
    )
  })

  it('serialises writes that arrive while an earlier one is in flight', async () => {
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

    await Promise.all([
      usePrefsStore.getState().update({ theme: 'dark' }),
      usePrefsStore.getState().update({ grid_size: 8 }),
    ])

    expect(saved).toEqual(stored({ theme: 'dark', grid_size: 8 }))
  })

  it('keeps a change asked for after a reset', async () => {
    let saved: Prefs = stored({ recent_limit: 5 })
    const transport = createMockTransport({
      'prefs.get': () => saved,
      'prefs.set': ({ prefs }) => {
        saved = prefs
        return prefs
      },
    })
    setTransport(transport)
    await usePrefsStore.getState().load()

    // Three rapid actions: a field, the reset button, then another field.
    const first = usePrefsStore.getState().update({ theme: 'dark' })
    const cleared = usePrefsStore.getState().reset()
    const second = usePrefsStore.getState().update({ grid_size: 8 })
    await Promise.all([first, cleared, second])

    // The reset discards what preceded it; the later change lands on top of
    // the defaults rather than being swallowed by the reset's turn in the queue.
    expect(saved).toEqual({ ...DEFAULT_PREFS, grid_size: 8 })
    expect(usePrefsStore.getState().prefs).toEqual({
      ...DEFAULT_PREFS,
      grid_size: 8,
    })
    expect(
      transport.calls.filter((entry) => entry.method === 'prefs.set'),
    ).toHaveLength(1)
  })

  it('keeps a change asked for a tick after a reset', async () => {
    let saved: Prefs = stored({ recent_limit: 5 })
    const transport = createMockTransport({
      'prefs.get': () => saved,
      'prefs.set': ({ prefs }) => {
        saved = prefs
        return prefs
      },
    })
    setTransport(transport)
    await usePrefsStore.getState().load()

    const first = usePrefsStore.getState().update({ theme: 'dark' })
    const cleared = usePrefsStore.getState().reset()
    // A tick's separation must not change which call the change belongs after.
    await Promise.resolve()
    const second = usePrefsStore.getState().update({ grid_size: 8 })
    await Promise.all([first, cleared, second])

    expect(saved).toEqual({ ...DEFAULT_PREFS, grid_size: 8 })
  })

  it('discards the changes a reset supersedes', async () => {
    let saved: Prefs = stored({ recent_limit: 5 })
    const transport = createMockTransport({
      'prefs.get': () => saved,
      'prefs.set': ({ prefs }) => {
        saved = prefs
        return prefs
      },
    })
    setTransport(transport)
    await usePrefsStore.getState().load()

    await Promise.all([
      usePrefsStore.getState().update({ theme: 'dark' }),
      usePrefsStore.getState().update({ grid_size: 8 }),
      usePrefsStore.getState().reset(),
    ])

    expect(saved).toEqual(DEFAULT_PREFS)
    expect(
      transport.calls.filter((entry) => entry.method === 'prefs.set'),
    ).toHaveLength(1)
  })

  it('applies a reset requested while a write is in flight', async () => {
    let saved: Prefs = stored({ recent_limit: 5 })
    let release = () => undefined as void
    const write = new Promise<void>((resolve) => {
      release = () => resolve()
    })
    let onTheWire = () => undefined as void
    const started = new Promise<void>((resolve) => {
      onTheWire = () => resolve()
    })
    let gated = true
    const transport = createMockTransport({
      'prefs.get': () => saved,
      'prefs.set': async ({ prefs }) => {
        if (gated) {
          gated = false
          onTheWire()
          await write
        }
        saved = prefs
        return prefs
      },
    })
    setTransport(transport)
    await usePrefsStore.getState().load()

    const inFlight = usePrefsStore.getState().update({ theme: 'dark' })
    await started
    // The reset and the field edit arrive while the first write is on the wire.
    const cleared = usePrefsStore.getState().reset()
    const after = usePrefsStore.getState().update({ grid_size: 8 })
    release()
    await Promise.all([inFlight, cleared, after])

    expect(saved).toEqual({ ...DEFAULT_PREFS, grid_size: 8 })
    expect(
      transport.calls.filter((entry) => entry.method === 'prefs.set'),
    ).toHaveLength(2)
  })

  it('refuses to write when the stored record cannot be read', async () => {
    const transport = createMockTransport({
      'prefs.get': () => {
        throw new Error('config directory is unreadable')
      },
    })
    setTransport(transport)

    await usePrefsStore.getState().update({ theme: 'dark' })

    expect(transport.calls.map((entry) => entry.method)).toEqual(['prefs.get'])
    expect(usePrefsStore.getState().prefs.theme).toBe(DEFAULT_PREFS.theme)
    expect(useUiStore.getState().toasts.at(-1)?.kind).toBe('error')
  })

  it('resets to the defaults even before a successful load', async () => {
    const transport = createMockTransport({
      'prefs.get': () => {
        throw new Error('config directory is unreadable')
      },
      'prefs.set': ({ prefs }) => prefs,
    })
    setTransport(transport)

    await usePrefsStore.getState().reset()

    expect(transport.calls.at(-1)).toEqual({
      method: 'prefs.set',
      params: { prefs: DEFAULT_PREFS },
    })
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
