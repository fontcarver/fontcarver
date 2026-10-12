import { afterEach, describe, expect, it, vi } from 'vitest'

import {
  setTransport,
  type StyleBatchParams,
  type StyleBatchResult,
} from '../api'
import { createMockTransport, noChange } from '../test/mockTransport'
import type { FilterTarget } from './contracts'
import { createFilterSession, useFilterPreview } from './filterSession'
import { usePluginStore } from './pluginStore'
import { translateFilter } from './registry'

const pluginId = translateFilter.manifest.id
const target: FilterTarget = {
  doc: 3,
  gid: 9,
  glyph: 'A',
  layer: 'regular',
  master: 'm1',
}
const label = 'Translate outline'
const idle = { phase: 'idle', preview: null, error: null }

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (error: Error) => void
  const promise = new Promise<T>((yes, no) => {
    resolve = yes
    reject = no
  })
  return { promise, resolve, reject }
}

/** Stateful transport double: preview has no transaction; apply creates one undo entry. */
function backend() {
  const original = {
    points: [
      [10, 20],
      [40, 60],
    ],
    advance: 500,
  }
  let outline = structuredClone(original)
  let generation = 17
  const history: { outline: typeof original; label: string }[] = []
  const path = (value: typeof original) =>
    value.points
      .map(([x, y], index) => `${index ? 'L' : 'M'}${x} ${y}`)
      .join(' ')
  const execute = vi.fn(
    async (request: StyleBatchParams): Promise<StyleBatchResult> => {
      if (!request.dry_run && request.expected_generation !== generation)
        throw new Error('style preview is stale: document changed')
      const [dx, dy] = request.options.translate
      const after = {
        ...outline,
        points: outline.points.map(([x, y]) => [x + dx, y + dy]),
      }
      const result: StyleBatchResult = {
        generation,
        glyph_count: 1,
        layer_count: 1,
        decomposed_layers: 0,
        approximated_layers: 0,
        approximated_sources: [],
        examples: [
          {
            glyph: target.glyph,
            master: 'Regular',
            before: path(outline),
            after: path(after),
            before_width: outline.advance,
            after_width: after.advance,
            bounds: [10, 20, 140, 260],
          },
        ],
        change: request.dry_run ? null : noChange([target.gid]),
      }
      if (!request.dry_run) {
        history.push({
          outline: structuredClone(outline),
          label: request.label,
        })
        outline = after
        generation += 1
      }
      return result
    },
  )
  const transport = createMockTransport({
    'style.batch': execute,
    'doc.undo': () => {
      const previous = history.pop()
      if (previous) {
        outline = previous.outline
        generation += 1
      }
      return noChange([target.gid])
    },
  })
  const settle = vi.fn(async (): Promise<void> => undefined)
  let enabled = true
  const session = createFilterSession({
    execute,
    settle,
    isEnabled: () => enabled,
  })
  return {
    session,
    execute,
    settle,
    transport,
    original,
    history,
    get outline() {
      return outline
    },
    get generation() {
      return generation
    },
    disable: () => {
      enabled = false
    },
    edit: () => {
      outline.points[0] = [99, 100]
      generation += 1
    },
  }
}

afterEach(() => {
  useFilterPreview.getState().cancel()
  usePluginStore.getState().remove(pluginId)
  setTransport(undefined)
})

describe('filter preview lifecycle', () => {
  it('captures the exact target and parameters, settles edits, and previews without mutation', async () => {
    const source = backend()
    const waiting = deferred<void>()
    source.settle.mockImplementationOnce(() => waiting.promise)
    const requestedTarget = { ...target }
    const parameters = { dx: 100, dy: 200 }
    const pending = source.session
      .getState()
      .prepare(pluginId, requestedTarget, parameters, label)
    requestedTarget.gid = 42
    parameters.dx = 999
    expect(source.session.getState().phase).toBe('loading')
    expect(source.execute).not.toHaveBeenCalled()
    waiting.resolve()
    await pending
    expect(source.execute).toHaveBeenCalledExactlyOnceWith({
      doc: target.doc,
      glyphs: [target.gid],
      master: target.master,
      options: {
        scale: [1, 1],
        translate: [100, 200],
        oblique: 0,
        weight: [0, 0],
      },
      scale_advance: false,
      center: false,
      backup: false,
      dry_run: true,
      expected_generation: null,
      label,
    })
    expect(source.session.getState()).toMatchObject({
      phase: 'ready',
      error: null,
      preview: {
        target,
        generation: 17,
        overlays: [{ kind: 'path', path: 'M110 220 L140 260' }],
      },
    })
    expect(source.outline).toEqual(source.original)
    expect(source.generation).toBe(17)
    expect(source.history).toEqual([])
  })

  it('applies the reviewed generation and plan once, and backend doc.undo restores the original', async () => {
    const source = backend()
    await source.session
      .getState()
      .prepare(pluginId, target, { dx: 100, dy: 200 }, label)
    const preview = source.session.getState().preview!
    expect(await source.session.getState().apply()).toBe(true)
    expect(source.execute).toHaveBeenLastCalledWith({
      ...preview.plan,
      dry_run: false,
      expected_generation: 17,
    })
    expect(source.outline).toEqual({
      points: [
        [110, 220],
        [140, 260],
      ],
      advance: 500,
    })
    expect(source.history).toEqual([{ outline: source.original, label }])
    expect(source.session.getState()).toMatchObject(idle)
    expect(await source.session.getState().apply()).toBe(false)
    expect(source.execute).toHaveBeenCalledTimes(2)
    await source.transport.call('doc.undo', { doc: target.doc })
    expect(source.transport.calls).toContainEqual({
      method: 'doc.undo',
      params: { doc: target.doc },
    })
    expect(source.outline).toEqual(source.original)
    expect(source.history).toEqual([])
  })

  it('rejects stale apply with the reviewed generation and clears its overlays', async () => {
    const source = backend()
    await source.session.getState().prepare(pluginId, target, { dx: 1 }, label)
    source.edit()
    const edited = structuredClone(source.outline)
    expect(await source.session.getState().apply()).toBe(false)
    expect(source.execute).toHaveBeenLastCalledWith(
      expect.objectContaining({ dry_run: false, expected_generation: 17 }),
    )
    expect(source.session.getState()).toMatchObject({
      phase: 'error',
      preview: null,
      error: { key: 'plugins.stale' },
    })
    expect(source.outline).toEqual(edited)
    expect(source.history).toEqual([])
  })

  it.each(['resolve', 'reject'] as const)(
    'cancel ignores a delayed preview %s and prevents apply',
    async (outcome) => {
      const source = backend()
      const pending = deferred<StyleBatchResult>()
      const result = await source.execute({
        ...translateFilter.plan(target, { dx: 1 }, label),
        dry_run: true,
        expected_generation: null,
      })
      source.execute.mockClear().mockImplementationOnce(() => pending.promise)
      const prepared = source.session
        .getState()
        .prepare(pluginId, target, { dx: 1 }, label)
      await vi.waitFor(() => expect(source.execute).toHaveBeenCalledOnce())
      source.session.getState().cancel()
      if (outcome === 'resolve') pending.resolve(result)
      else pending.reject(new Error('late backend failure'))
      await prepared
      expect(source.session.getState()).toMatchObject(idle)
      expect(await source.session.getState().apply()).toBe(false)
      expect(source.execute).toHaveBeenCalledOnce()
      expect(source.outline).toEqual(source.original)
      expect(source.history).toEqual([])
    },
  )

  it('the latest preview wins when an older request returns later', async () => {
    const source = backend()
    const earlier = deferred<StyleBatchResult>()
    const result = await source.execute({
      ...translateFilter.plan(target, { dx: 1 }, label),
      dry_run: true,
      expected_generation: null,
    })
    source.execute.mockClear().mockImplementationOnce(() => earlier.promise)
    const first = source.session
      .getState()
      .prepare(pluginId, target, { dx: 1 }, label)
    await vi.waitFor(() => expect(source.execute).toHaveBeenCalledOnce())
    await source.session
      .getState()
      .prepare(pluginId, target, { dy: 2 }, 'Latest')
    const latest = source.session.getState().preview
    earlier.resolve(result)
    await first
    expect(source.session.getState().preview).toBe(latest)
    expect(latest?.plan).toMatchObject({
      label: 'Latest',
      options: { translate: [0, 2] },
    })
    expect(source.history).toEqual([])
  })

  it.each(['cancel', 'disable'] as const)(
    '%s during preview settlement releases loading without dispatching a request',
    async (action) => {
      const source = backend()
      const pending = deferred<void>()
      source.settle.mockImplementationOnce(() => pending.promise)
      const prepared = source.session
        .getState()
        .prepare(pluginId, target, { dx: 1 }, label)
      if (action === 'disable') source.disable()
      else source.session.getState().cancel()
      pending.resolve()
      await prepared
      expect(source.execute).not.toHaveBeenCalled()
      expect(source.session.getState()).toMatchObject(idle)
    },
  )

  it('disabling before apply prevents mutation and clears the preview', async () => {
    const source = backend()
    await source.session.getState().prepare(pluginId, target, { dx: 1 }, label)
    source.disable()
    expect(await source.session.getState().apply()).toBe(false)
    expect(source.execute).toHaveBeenCalledOnce()
    expect(source.session.getState()).toMatchObject({
      phase: 'error',
      preview: null,
      error: { key: 'plugins.notEnabled' },
    })
    expect(source.outline).toEqual(source.original)
  })

  it('disabling during apply settlement prevents dispatching the mutation', async () => {
    const source = backend()
    await source.session.getState().prepare(pluginId, target, { dx: 1 }, label)
    const pending = deferred<void>()
    source.settle.mockImplementationOnce(() => pending.promise)
    const applying = source.session.getState().apply()
    source.disable()
    pending.resolve()
    expect(await applying).toBe(false)
    expect(source.execute).toHaveBeenCalledOnce()
    expect(source.session.getState().preview).toBeNull()
    expect(source.history).toEqual([])
  })

  it.each(['prepare', 'apply'] as const)(
    'surfaces backend failure during %s and releases the preview',
    async (stage) => {
      const source = backend()
      if (stage === 'apply')
        await source.session
          .getState()
          .prepare(pluginId, target, { dx: 1 }, label)
      source.execute.mockRejectedValueOnce(new Error('Document closed'))
      if (stage === 'prepare')
        await source.session
          .getState()
          .prepare(pluginId, target, { dx: 1 }, label)
      else expect(await source.session.getState().apply()).toBe(false)
      expect(source.session.getState()).toMatchObject({
        phase: 'error',
        preview: null,
        error: { key: 'plugins.failed', detail: 'Document closed' },
      })
      expect(source.history).toEqual([])
    },
  )

  it.each<Record<string, number>>([
    {},
    { dx: 0, dy: 0 },
    { dx: Number.NaN },
    { dy: Infinity },
    { dx: 32768 },
    { dy: -32768 },
  ])(
    'rejects invalid or zero translation %j before transport',
    async (parameters) => {
      const source = backend()
      await source.session
        .getState()
        .prepare(pluginId, target, parameters, label)
      expect(source.session.getState()).toMatchObject({
        phase: 'error',
        preview: null,
        error: { key: 'plugins.invalidOffset' },
      })
      expect(source.execute).not.toHaveBeenCalled()
      expect(source.settle).not.toHaveBeenCalled()
    },
  )

  it.each(['disable', 'remove'] as const)(
    'plugin %s immediately releases the shared preview and prevents apply',
    async (action) => {
      const source = backend()
      setTransport(source.transport)
      usePluginStore.getState().install(pluginId)
      usePluginStore.getState().enable(pluginId)
      await useFilterPreview
        .getState()
        .prepare(pluginId, target, { dx: 1 }, label)
      expect(useFilterPreview.getState().phase).toBe('ready')
      usePluginStore.getState()[action](pluginId)
      expect(useFilterPreview.getState()).toMatchObject(idle)
      expect(await useFilterPreview.getState().apply()).toBe(false)
      expect(source.execute).toHaveBeenCalledOnce()
      expect(source.outline).toEqual(source.original)
      expect(source.history).toEqual([])
    },
  )
})
