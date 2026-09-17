import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { ShapedRun } from '../../api'
import { createMockTransport } from '../../test/mockTransport'
import { cellFor } from '../../test/glyphFixtures'
import {
  TextShapeCoordinator,
  type TextRequestContext,
  type TextShapeState,
} from './shapeCoordinator'

const context = (text: string): TextRequestContext => ({
  doc: 1,
  generation: 1,
  unitsPerEm: 1_000,
  master: null,
  text,
  features: [],
  direction: 'ltr',
  script: null,
  language: null,
})

const shaped = (text: string, stale: boolean): ShapedRun => ({
  glyphs: [
    {
      gid: text.charCodeAt(0),
      cluster: 0,
      x_advance: 500,
      y_advance: 0,
      x_offset: 0,
      y_offset: 0,
    },
  ],
  names: [text],
  warnings: [],
  stale,
  units_per_em: 1_000,
  size: null,
})

const deferred = <T>() => {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((done) => {
    resolve = done
  })
  return { promise, resolve }
}

describe('TextShapeCoordinator', () => {
  beforeEach(() => vi.useFakeTimers())
  afterEach(() => vi.useRealTimers())

  it('polls a stale run until fresh without waiting for another event', async () => {
    let request = 0
    const transport = createMockTransport({
      'text.shape': ({ text }) => shaped(text, request++ === 0),
      'glyphs.search': ({ query }) => [cellFor(8, { name: query })],
    })
    const states: TextShapeState[] = []
    const coordinator = new TextShapeCoordinator(
      transport,
      (state) => states.push(state),
      { debounceMs: 150, pollDelaysMs: [50, 100] },
    )

    coordinator.update(context('A'))
    await vi.advanceTimersByTimeAsync(150)
    expect(states.at(-1)?.stale).toBe(true)
    await vi.advanceTimersByTimeAsync(50)

    expect(states.at(-1)?.stale).toBe(false)
    expect(
      transport.calls.filter((call) => call.method === 'text.shape'),
    ).toHaveLength(2)
  })

  it('cancels debounce and stale polling when disposed', async () => {
    const transport = createMockTransport({
      'text.shape': ({ text }) => shaped(text, true),
      'glyphs.search': ({ query }) => [cellFor(8, { name: query })],
    })
    const coordinator = new TextShapeCoordinator(transport, vi.fn(), {
      debounceMs: 150,
      pollDelaysMs: [50],
    })

    coordinator.update(context('A'))
    await vi.advanceTimersByTimeAsync(150)
    coordinator.dispose()
    await vi.advanceTimersByTimeAsync(1_000)

    expect(
      transport.calls.filter((call) => call.method === 'text.shape'),
    ).toHaveLength(1)
  })

  it('never lets an older response overwrite a newer request', async () => {
    const old = deferred<ShapedRun>()
    const transport = createMockTransport({
      'text.shape': ({ text }) =>
        text === 'old' ? old.promise : shaped(text, false),
      'glyphs.search': ({ query }) => [cellFor(8, { name: query })],
    })
    const states: TextShapeState[] = []
    const coordinator = new TextShapeCoordinator(
      transport,
      (state) => states.push(state),
      { debounceMs: 150, pollDelaysMs: [50] },
    )

    coordinator.update(context('old'))
    await vi.advanceTimersByTimeAsync(150)
    coordinator.update(context('new'))
    await vi.advanceTimersByTimeAsync(150)
    expect(states.at(-1)?.context.text).toBe('new')

    old.resolve(shaped('old', false))
    await vi.runAllTimersAsync()
    expect(states.at(-1)?.context.text).toBe('new')
  })

  it('keeps the last good layout for the same text while recompiling', async () => {
    let request = 0
    const transport = createMockTransport({
      'text.shape': ({ text }) => {
        const result = shaped(text, request > 0)
        result.glyphs[0]!.x_advance = request++ === 0 ? 500 : 900
        return result
      },
      'glyphs.search': ({ query }) => [cellFor(8, { name: query })],
    })
    const states: TextShapeState[] = []
    const coordinator = new TextShapeCoordinator(
      transport,
      (state) => states.push(state),
      { debounceMs: 150, pollDelaysMs: [1_000] },
    )

    coordinator.update(context('A'))
    await vi.advanceTimersByTimeAsync(150)
    coordinator.notifyDocumentEvent({
      doc: 1,
      generation: 2,
      change: {
        glyphs: [8],
        font_info: false,
        masters: false,
        layout: false,
        feature_source: false,
        glyph_order: false,
        metrics: false,
        layers: false,
        guides: false,
      },
      undo_label: 'Edit',
      redo_label: null,
      dirty: true,
    })
    await vi.advanceTimersByTimeAsync(150)

    expect(states.at(-1)).toMatchObject({
      stale: true,
      showingPrevious: true,
      context: { generation: 2 },
      layout: { end: { x: 500 } },
    })
    coordinator.dispose()
  })

  it('deduplicates compile failures by document generation', async () => {
    const transport = createMockTransport({
      'text.shape': ({ text }) => ({
        ...shaped(text, true),
        warnings: ['compile-failed: broken outline'],
      }),
      'glyphs.search': ({ query }) => [cellFor(8, { name: query })],
    })
    const states: TextShapeState[] = []
    const coordinator = new TextShapeCoordinator(
      transport,
      (state) => states.push(state),
      { debounceMs: 150, pollDelaysMs: [50] },
    )

    coordinator.update(context('A'))
    await vi.advanceTimersByTimeAsync(150)
    coordinator.update(context('B'))
    await vi.advanceTimersByTimeAsync(150)
    expect(states.map((state) => state.newCompileFailure)).toEqual([
      'compile-failed: broken outline',
      null,
    ])

    coordinator.notifyDocumentEvent({
      doc: 1,
      generation: 2,
      change: {
        glyphs: [],
        font_info: false,
        masters: false,
        layout: false,
        feature_source: false,
        glyph_order: false,
        metrics: false,
        layers: false,
        guides: false,
      },
      undo_label: null,
      redo_label: null,
      dirty: true,
    })
    await vi.advanceTimersByTimeAsync(150)
    expect(states.at(-1)?.newCompileFailure).toBe(
      'compile-failed: broken outline',
    )
  })

  it('aborts a superseded resolver before it launches more pooled work', async () => {
    let release!: () => void
    const held = new Promise<void>((resolve) => {
      release = resolve
    })
    const transport = createMockTransport({
      'glyphs.search': async ({ query }) => {
        await held
        return [cellFor(8, { name: query })]
      },
    })
    const coordinator = new TextShapeCoordinator(transport, vi.fn(), {
      debounceMs: 150,
    })

    const names = Array.from({ length: 20 }, (_, index) => `/g${index}`).join(
      ' ',
    )
    coordinator.update(context(names))
    await vi.advanceTimersByTimeAsync(150)
    await Promise.resolve()
    expect(
      transport.calls.filter((call) => call.method === 'glyphs.search'),
    ).toHaveLength(8)

    coordinator.update(context('new'))
    release()
    await Promise.resolve()
    await Promise.resolve()
    expect(
      transport.calls.filter((call) => call.method === 'glyphs.search'),
    ).toHaveLength(8)
    coordinator.dispose()
  })
})
