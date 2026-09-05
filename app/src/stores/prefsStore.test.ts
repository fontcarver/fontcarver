import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { setTransport, type Prefs } from '../api'
import { createMockTransport, type MockTransport } from '../test/mockTransport'

import { DEFAULT_PREFS, initialPrefsState, usePrefsStore } from './prefsStore'
import { initialUiState, useUiStore } from './uiStore'

function stored(overrides: Partial<Prefs> = {}): Prefs {
  return { ...DEFAULT_PREFS, ...overrides }
}

/** The records `prefs.set` was called with, in the order they went out. */
function writes(transport: MockTransport): Prefs[] {
  return transport.calls
    .filter((entry) => entry.method === 'prefs.set')
    .map((entry) => (entry.params as { prefs: Prefs }).prefs)
}

/** The error toasts raised so far. */
function errors(): string[] {
  return useUiStore
    .getState()
    .toasts.filter((toast) => toast.kind === 'error')
    .map((toast) => toast.message)
}

/**
 * Watches for promise rejections nobody handled.
 *
 * The View menu calls `update` without awaiting it, so a queue that let a
 * rejected write escape would crash the process rather than show a toast.
 */
function watchUnhandledRejections(): { settle: () => Promise<unknown[]> } {
  const node = globalThis as unknown as {
    process?: {
      on: (
        event: 'unhandledRejection',
        listener: (why: unknown) => void,
      ) => void
      off: (
        event: 'unhandledRejection',
        listener: (why: unknown) => void,
      ) => void
    }
  }
  const seen: unknown[] = []
  const record = (why: unknown): void => {
    seen.push(why)
  }
  node.process?.on('unhandledRejection', record)
  return {
    settle: async () => {
      // Node reports an unhandled rejection once the turn that made it ends.
      await new Promise((resolve) => setTimeout(resolve, 0))
      await new Promise((resolve) => setTimeout(resolve, 0))
      node.process?.off('unhandledRejection', record)
      return seen
    },
  }
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

  it('reads once for two changes that race the first read', async () => {
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

    // One read — the second change builds on what the first write confirmed —
    // then a write per change, neither of them losing the other's patch nor
    // the stored fields.
    expect(transport.calls.map((entry) => entry.method)).toEqual([
      'prefs.get',
      'prefs.set',
      'prefs.set',
    ])
    expect(writes(transport)).toEqual([
      stored({ recent_limit: 5, autosave_seconds: 120, theme: 'dark' }),
      stored({
        recent_limit: 5,
        autosave_seconds: 120,
        theme: 'dark',
        grid_size: 8,
      }),
    ])
    expect(usePrefsStore.getState().prefs).toEqual(
      stored({
        recent_limit: 5,
        autosave_seconds: 120,
        theme: 'dark',
        grid_size: 8,
      }),
    )
  })

  it('keeps a slow start-up read from undoing a change made while it ran', async () => {
    let saved: Prefs = stored({ recent_limit: 5 })
    let slow = true
    const transport = createMockTransport({
      'prefs.get': async () => {
        // The kernel reads the file when the call arrives, not when it replies,
        // and the first read of the session is the one that takes a while.
        const record = saved
        if (slow) {
          slow = false
          await new Promise((resolve) => setTimeout(resolve, 25))
        }
        return record
      },
      'prefs.set': ({ prefs }) => {
        saved = prefs
        return prefs
      },
    })
    setTransport(transport)

    // The shell's start-up read, and a View-menu theme click before it lands.
    const startUp = usePrefsStore.getState().load()
    const changed = usePrefsStore.getState().update({ theme: 'dark' })
    await Promise.all([startUp, changed])
    await usePrefsStore.getState().update({ grid_size: 8 })

    // The read takes its turn in the same queue as the writes, so it lands
    // before them instead of coming back afterwards with the record it fetched
    // beforehand: one read, and neither change is undone by it.
    expect(transport.calls.map((entry) => entry.method)).toEqual([
      'prefs.get',
      'prefs.set',
      'prefs.set',
    ])
    expect(writes(transport)).toEqual([
      stored({ recent_limit: 5, theme: 'dark' }),
      stored({ recent_limit: 5, theme: 'dark', grid_size: 8 }),
    ])
    expect(saved).toEqual(
      stored({ recent_limit: 5, theme: 'dark', grid_size: 8 }),
    )
    expect(usePrefsStore.getState().confirmed).toEqual(saved)
    expect(usePrefsStore.getState().prefs).toEqual(saved)
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

    // One write per action, in the order the user acted in: the reset lands
    // after the field it supersedes and before the field asked for after it,
    // which lands on top of the defaults rather than being swallowed.
    expect(writes(transport)).toEqual([
      stored({ recent_limit: 5, theme: 'dark' }),
      DEFAULT_PREFS,
      { ...DEFAULT_PREFS, grid_size: 8 },
    ])
    expect(saved).toEqual({ ...DEFAULT_PREFS, grid_size: 8 })
    expect(usePrefsStore.getState().prefs).toEqual({
      ...DEFAULT_PREFS,
      grid_size: 8,
    })
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

    expect(writes(transport)).toEqual([
      stored({ recent_limit: 5, theme: 'dark' }),
      DEFAULT_PREFS,
      { ...DEFAULT_PREFS, grid_size: 8 },
    ])
    expect(saved).toEqual({ ...DEFAULT_PREFS, grid_size: 8 })
  })

  it('lets a reset asked for last undo the changes before it', async () => {
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

    expect(writes(transport)).toEqual([
      stored({ recent_limit: 5, theme: 'dark' }),
      stored({ recent_limit: 5, theme: 'dark', grid_size: 8 }),
      DEFAULT_PREFS,
    ])
    expect(saved).toEqual(DEFAULT_PREFS)
    expect(usePrefsStore.getState().prefs).toEqual(DEFAULT_PREFS)
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

    expect(writes(transport)).toEqual([
      stored({ recent_limit: 5, theme: 'dark' }),
      DEFAULT_PREFS,
      { ...DEFAULT_PREFS, grid_size: 8 },
    ])
    expect(saved).toEqual({ ...DEFAULT_PREFS, grid_size: 8 })
    expect(usePrefsStore.getState().prefs).toEqual({
      ...DEFAULT_PREFS,
      grid_size: 8,
    })
  })

  it('builds the writes after a rejected one on what the kernel really holds', async () => {
    let saved: Prefs = stored({ recent_limit: 5 })
    let attempts = 0
    const transport = createMockTransport({
      'prefs.get': () => saved,
      'prefs.set': ({ prefs }) => {
        attempts += 1
        // The reset is the write the kernel drops.
        if (attempts === 2) throw new Error('kernel unreachable')
        saved = prefs
        return prefs
      },
    })
    setTransport(transport)
    await usePrefsStore.getState().load()

    const first = usePrefsStore.getState().update({ theme: 'dark' })
    const cleared = usePrefsStore.getState().reset()
    const after = usePrefsStore.getState().update({ grid_size: 8 })
    await Promise.all([first, cleared, after])

    // The dropped reset takes nothing else with it: the change after it lands
    // on the record the kernel confirmed, which is still the first write's.
    expect(writes(transport)).toEqual([
      stored({ recent_limit: 5, theme: 'dark' }),
      DEFAULT_PREFS,
      stored({ recent_limit: 5, theme: 'dark', grid_size: 8 }),
    ])
    expect(saved).toEqual(
      stored({ recent_limit: 5, theme: 'dark', grid_size: 8 }),
    )
    // What the interface shows is what the kernel stored, and the shell has not
    // been left wearing the theme the failed reset asked for.
    expect(usePrefsStore.getState().prefs).toEqual(saved)
    expect(useUiStore.getState().theme).toBe('dark')
    expect(errors()).toEqual(['kernel unreachable'])
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

  it('reports a failed write instead of rejecting what it handed back', async () => {
    const transport = createMockTransport({
      'prefs.get': () => stored(),
      'prefs.set': () => {
        throw new Error('config directory is read-only')
      },
    })
    setTransport(transport)
    await usePrefsStore.getState().load()

    const unhandled = watchUnhandledRejections()
    // Nobody awaits these: the View menu does not either.
    usePrefsStore.getState().update({ theme: 'dark' })
    usePrefsStore.getState().reset()
    await expect(
      usePrefsStore.getState().update({ grid_size: 12 }),
    ).resolves.toBeUndefined()

    // Three failed writes, three toasts, and nothing that could crash a
    // caller that only wanted the toast.
    expect(await unhandled.settle()).toEqual([])
    expect(errors()).toHaveLength(3)
    expect(usePrefsStore.getState().prefs).toEqual(stored())
  })
})
