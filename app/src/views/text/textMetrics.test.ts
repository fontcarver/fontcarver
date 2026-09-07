import { afterEach, describe, expect, it, vi } from 'vitest'

import { call, setTransport, type EditApplyParams } from '../../api'
import type { Scheduler } from '../../editor/editSession'
import {
  createEditingTransport,
  createMockTransport,
  noChange,
  type MockTransport,
} from '../../test/mockTransport'

import {
  metricShortcut,
  TextMetricsRunner,
  type MetricShortcutEvent,
  type TextMetricsPreview,
} from './textMetrics'

function deferred(): { promise: Promise<void>; resolve: () => void } {
  let resolve: () => void = () => undefined
  const promise = new Promise<void>((done) => {
    resolve = done
  })
  return { promise, resolve }
}

function manualScheduler(): { scheduler: Scheduler; flush: () => void } {
  let callback: (() => void) | null = null
  let handle = 0
  return {
    scheduler: {
      request(next) {
        callback = next
        handle += 1
        return handle
      },
      cancel(cancelled) {
        if (cancelled === handle) callback = null
      },
    },
    flush() {
      const queued = callback
      callback = null
      queued?.()
    },
  }
}

function applies(mock: MockTransport): EditApplyParams[] {
  return mock.calls
    .filter((entry) => entry.method === 'doc.edit_apply')
    .map((entry) => entry.params as EditApplyParams)
}

function runner(
  mock: MockTransport,
  overrides: Partial<ConstructorParameters<typeof TextMetricsRunner>[0]> = {},
): {
  metrics: TextMetricsRunner
  previews: TextMetricsPreview[]
  ended: ReturnType<typeof vi.fn>
  errors: unknown[]
  frames: ReturnType<typeof manualScheduler>
} {
  setTransport(mock)
  const previews: TextMetricsPreview[] = []
  const ended = vi.fn()
  const errors: unknown[] = []
  const frames = manualScheduler()
  return {
    previews,
    ended,
    errors,
    frames,
    metrics: new TextMetricsRunner({
      doc: 1,
      glyph: 'A',
      layer: 'layer-1',
      start: { width: 600, lsb: 40, rsb: 60 },
      label: 'Set metrics',
      scheduler: frames.scheduler,
      onPreview: (preview) => previews.push(preview),
      onEnd: ended,
      onError: (error) => errors.push(error),
      ...overrides,
    }),
  }
}

afterEach(() => {
  setTransport(undefined)
  vi.useRealTimers()
  vi.restoreAllMocks()
})

describe('metricShortcut', () => {
  const event = (
    overrides: Partial<MetricShortcutEvent>,
  ): MetricShortcutEvent => ({
    key: 'ArrowLeft',
    altKey: false,
    shiftKey: false,
    ctrlKey: false,
    metaKey: false,
    ...overrides,
  })

  it('maps platform Mod+arrows to the stated sides and signs', () => {
    expect(metricShortcut(event({ ctrlKey: true }), false)).toEqual({
      side: 'lsb',
      delta: -1,
    })
    expect(
      metricShortcut(event({ key: 'ArrowRight', ctrlKey: true }), false),
    ).toEqual({ side: 'rsb', delta: 1 })
    expect(metricShortcut(event({ metaKey: true }), true)).toEqual({
      side: 'lsb',
      delta: -1,
    })
    expect(metricShortcut(event({ ctrlKey: true }), true)).toBeNull()
  })

  it('uses Shift for ten and macOS Meta+Ctrl for one hundred', () => {
    expect(
      metricShortcut(
        event({ key: 'ArrowRight', ctrlKey: true, shiftKey: true }),
        false,
      ),
    ).toEqual({ side: 'rsb', delta: 10 })
    expect(
      metricShortcut(event({ metaKey: true, ctrlKey: true }), true),
    ).toEqual({ side: 'lsb', delta: -100 })
    // Linux Ctrl is already Mod, so Ctrl alone remains the one-unit step.
    expect(metricShortcut(event({ ctrlKey: true }), false)?.delta).toBe(-1)
  })

  it('leaves Alt, unmodified arrows, and other keys unclaimed', () => {
    expect(
      metricShortcut(event({ ctrlKey: true, altKey: true }), false),
    ).toBeNull()
    expect(metricShortcut(event({}), false)).toBeNull()
    expect(
      metricShortcut(event({ key: 'ArrowUp', ctrlKey: true }), false),
    ).toBeNull()
  })
})

describe('TextMetricsRunner', () => {
  it('keeps alternating sides in one edit and one undo boundary', async () => {
    const mock = createEditingTransport()
    const { metrics, previews, frames, ended } = runner(mock)

    metrics.press('lsb', -1)
    frames.flush()
    await vi.waitFor(() => expect(applies(mock)).toHaveLength(1))
    metrics.press('rsb', 10)
    frames.flush()
    await metrics.end()
    await call('doc.undo', { doc: 1 })

    expect(mock.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_apply',
      'doc.edit_end',
      'doc.undo',
    ])
    expect(applies(mock).map((entry) => entry.command)).toEqual([
      {
        SetMetrics: {
          glyph: 'A',
          layer: 'layer-1',
          width: null,
          lsb: 39,
          rsb: 60,
        },
      },
      {
        SetMetrics: {
          glyph: 'A',
          layer: 'layer-1',
          width: null,
          lsb: 39,
          rsb: 70,
        },
      },
    ])
    expect(previews).toEqual([
      { width: 599, lsb: 39, rsb: 60, lsbDelta: -1 },
      { width: 609, lsb: 39, rsb: 70, lsbDelta: -1 },
    ])
    expect(ended).toHaveBeenCalledOnce()
    expect(metrics.active).toBe(false)
  })

  it('accumulates repeats while the controller gate delays edit_begin', async () => {
    const gate = deferred()
    const mock = createEditingTransport()
    const { metrics, previews, frames } = runner(mock, {
      after: gate.promise,
    })

    metrics.press('lsb', -1)
    metrics.press('lsb', -10)
    frames.flush()
    const ending = metrics.end()
    expect(mock.calls).toEqual([])
    expect(previews.at(-1)).toEqual({
      width: 589,
      lsb: 29,
      rsb: 60,
      lsbDelta: -11,
    })

    gate.resolve()
    await ending

    expect(mock.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_end',
    ])
    expect(applies(mock)[0]?.command).toEqual({
      SetMetrics: {
        glyph: 'A',
        layer: 'layer-1',
        width: null,
        lsb: 29,
        rsb: 60,
      },
    })
  })

  it('ends after 500 ms of silence', async () => {
    vi.useFakeTimers()
    const mock = createEditingTransport()
    const { metrics, ended } = runner(mock)

    metrics.press('rsb', 1)
    expect(metrics.active).toBe(true)
    await vi.advanceTimersByTimeAsync(499)
    expect(metrics.active).toBe(true)
    expect(ended).not.toHaveBeenCalled()

    await vi.advanceTimersByTimeAsync(1)

    expect(metrics.active).toBe(false)
    expect(ended).toHaveBeenCalledOnce()
    expect(mock.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_end',
    ])
  })

  it('returns one close promise and reports onEnd only after it resolves', async () => {
    const release = deferred()
    const mock = createEditingTransport({
      hold: { on: 'doc.edit_end', until: release.promise },
    })
    const { metrics, frames, ended } = runner(mock)
    metrics.press('lsb', -1)
    frames.flush()
    await vi.waitFor(() => expect(applies(mock)).toHaveLength(1))

    const first = metrics.end()
    const second = metrics.end()
    expect(first).toBe(second)
    expect(metrics.active).toBe(false)
    await vi.waitFor(() =>
      expect(mock.calls.at(-1)?.method).toBe('doc.edit_end'),
    )
    expect(ended).not.toHaveBeenCalled()

    release.resolve()
    await first
    expect(ended).toHaveBeenCalledOnce()
  })

  it('reports a failed session once and ends the run', async () => {
    const mock = createEditingTransport({
      failBegin: () => new Error('document closed'),
    })
    const { metrics, frames, errors, ended } = runner(mock)

    metrics.press('rsb', 1)
    frames.flush()
    await metrics.end()

    expect(errors.map(String)).toEqual(['Error: document closed'])
    expect(ended).toHaveBeenCalledOnce()
    expect(metrics.active).toBe(false)
  })

  it('supports an injected idle deadline', async () => {
    vi.useFakeTimers()
    const mock = createMockTransport({
      'doc.edit_begin': () => 101,
      'doc.edit_apply': () => noChange([42]),
      'doc.edit_end': () => null,
    })
    const { metrics, ended } = runner(mock, { timeout: 25 })

    metrics.press('lsb', -1)
    await vi.advanceTimersByTimeAsync(24)
    expect(ended).not.toHaveBeenCalled()
    await vi.advanceTimersByTimeAsync(1)
    expect(ended).toHaveBeenCalledOnce()
  })
})
