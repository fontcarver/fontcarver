import { act, renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { call, setTransport, type LayerOutline } from '../../api'
import * as editors from '../../editor/activeEditor'
import * as commands from '../../shell/commands'
import type { EditorController } from '../../editor/editorController'
import {
  initialEditorState,
  useEditorStore,
  type ActiveGlyph,
} from '../../stores/editorStore'
import {
  makeContour,
  makeLayer,
  makeLayerOutline,
  makePoint,
} from '../../test/fontFixtures'
import { createMockTransport, noChange } from '../../test/mockTransport'
import { useTextMetrics } from './useTextMetrics'
import { useUiStore } from '../../stores/uiStore'
import { initialTextState } from './textState'
import { suspendDocumentEditParticipants } from '../../editor/documentEditParticipants'

function deferred() {
  let resolve!: () => void
  const promise = new Promise<void>((done) => {
    resolve = done
  })
  return { promise, resolve }
}

const key = (side: 'left' | 'right', shift = false) => ({
  key: side === 'left' ? 'ArrowLeft' : 'ArrowRight',
  ctrlKey: true,
  metaKey: true,
  altKey: false,
  shiftKey: shift,
})

function setup(
  options: {
    preparation?: Promise<void>
    firstEnd?: Promise<void>
    read?: Promise<void>
    failRead?: boolean
  } = {},
) {
  let stored: LayerOutline = makeLayerOutline(
    makeLayer({
      width: 600,
      contours: [
        makeContour([makePoint(40, 0), makePoint(540, 0), makePoint(540, 700)]),
      ],
    }),
  )
  let token = 0
  let active: number | null = null
  const transport = createMockTransport({
    'doc.edit_begin': () => {
      active = ++token
      return token
    },
    'glyphs.outline': async () => {
      await options.read
      if (options.failRead) throw new Error('outline unavailable')
      return structuredClone(stored)
    },
    'doc.edit_apply': ({ token: applying, command }) => {
      if (applying !== active) throw new Error('edit overtaken')
      if (!('SetMetrics' in command)) throw new Error('unexpected command')
      const value = command.SetMetrics
      if (stored.bbox) {
        const delta = (value.lsb ?? stored.lsb) - stored.lsb
        stored = {
          ...stored,
          lsb: stored.lsb + delta,
          rsb: value.rsb ?? stored.rsb,
          width: stored.width + delta + (value.rsb ?? stored.rsb) - stored.rsb,
          bbox: [
            stored.bbox[0] + delta,
            stored.bbox[1],
            stored.bbox[2] + delta,
            stored.bbox[3],
          ],
        }
      }
      return noChange([7])
    },
    'doc.edit_end': async ({ token: ending }) => {
      if (ending === 1) await options.firstEnd
      if (ending !== active) throw new Error('end overtaken')
      active = null
      return null
    },
    'doc.apply': () => {
      active = null
      return noChange([7])
    },
  })
  setTransport(transport)
  const reload = vi.fn()
  const target = {
    doc: 1,
    gid: 7,
    outline: structuredClone(stored),
    master: 'm1',
    scene: { name: 'A' },
    reload,
  } as unknown as ActiveGlyph
  useEditorStore.setState({ ...initialEditorState(), activeGlyph: target })
  let finish = () => Promise.resolve()
  const preview = vi.fn()
  const editor = {
    // This mirrors the captured preparation contract: no late read of a
    // subsequently published metrics close may become its own prerequisite.
    prepareExternalEdit: () =>
      Promise.all([options.preparation, finish()]).then(() => undefined),
    setExternalPreview: preview,
  } as unknown as EditorController
  vi.spyOn(editors, 'activeEditor').mockReturnValue(editor)
  const hook = renderHook(() => useTextMetrics(true, 'Set metrics'))
  finish = hook.result.current.finish
  return { ...hook, transport, preview, reload, target, stored: () => stored }
}

afterEach(() => {
  setTransport(undefined)
  useUiStore.setState({ tabs: [], activeTab: null })
  useEditorStore.setState(initialEditorState())
  vi.useRealTimers()
  vi.restoreAllMocks()
})

describe('useTextMetrics ordering', () => {
  it('consumes spacing keys without edits while kerning mode locks spacing', async () => {
    const hook = setup()
    useUiStore.setState({
      tabs: [
        {
          id: 'locked',
          kind: 'glyph',
          doc: 1,
          textState: { ...initialTextState('m1'), kerningMode: 'locked' },
        },
      ],
      activeTab: 'locked',
    })
    act(() => expect(hook.result.current.onMetricKey(key('right'))).toBe(true))
    await hook.result.current.finish()
    expect(hook.transport.calls).toEqual([])
  })

  it('does not accept metrics shortcuts while the document is closing', async () => {
    const hook = setup()
    const resume = suspendDocumentEditParticipants(1)
    try {
      act(() => {
        hook.result.current.onMetricKey(key('left'))
      })
      await hook.result.current.finish()
      expect(hook.transport.calls).toEqual([])
    } finally {
      resume()
    }
  })
  it('preserves queued keys before an immediate ordinary command while old work closes', async () => {
    const preparation = deferred()
    const hook = setup({ preparation: preparation.promise })
    act(() => {
      hook.result.current.onMetricKey(key('left'))
      hook.result.current.onMetricKey(key('right', true))
    })
    const afterMetrics = hook.result.current.finish().then(() =>
      call('doc.apply', {
        doc: 1,
        command: { Batch: [] },
        label: 'Next command',
      }),
    )
    expect(hook.transport.calls).toHaveLength(0)
    preparation.resolve()
    await act(async () => {
      await afterMetrics
    })
    expect(hook.transport.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'glyphs.outline',
      'doc.edit_apply',
      'doc.edit_end',
      'doc.apply',
    ])
    expect(hook.stored()).toMatchObject({ width: 609, lsb: 39, rsb: 70 })
    hook.unmount()
  })

  it('retains a key arriving during edit_end and refreshes its baseline', async () => {
    const ending = deferred()
    const hook = setup({ firstEnd: ending.promise })
    act(() => {
      hook.result.current.onMetricKey(key('left'))
    })
    const first = hook.result.current.finish()
    await vi.waitFor(() =>
      expect(hook.transport.calls.at(-1)?.method).toBe('doc.edit_end'),
    )
    act(() => {
      hook.result.current.onMetricKey(key('left', true))
    })
    const all = hook.result.current.finish()
    expect(
      hook.transport.calls.filter((entry) => entry.method === 'doc.edit_begin'),
    ).toHaveLength(1)
    ending.resolve()
    await act(async () => {
      await first
      await all
    })
    expect(hook.stored()).toMatchObject({ width: 589, lsb: 29, rsb: 60 })
    expect(hook.transport.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'glyphs.outline',
      'doc.edit_apply',
      'doc.edit_end',
      'doc.edit_begin',
      'glyphs.outline',
      'doc.edit_apply',
      'doc.edit_end',
    ])
    expect(hook.reload).toHaveBeenCalledOnce()
    hook.unmount()
  })

  it('publishes an automatic idle close before reusing pending preparation', async () => {
    vi.useFakeTimers()
    const preparation = deferred()
    const ending = deferred()
    const hook = setup({
      preparation: preparation.promise,
      firstEnd: ending.promise,
    })
    act(() => {
      hook.result.current.onMetricKey(key('left'))
    })
    await vi.advanceTimersByTimeAsync(501)
    act(() => {
      hook.result.current.onMetricKey(key('right'))
    })
    const finished = hook.result.current.finish()
    preparation.resolve()
    await vi.advanceTimersByTimeAsync(0)
    expect(
      hook.transport.calls.filter((entry) => entry.method === 'doc.edit_begin'),
    ).toHaveLength(1)
    ending.resolve()
    await act(async () => {
      await finished
    })
    expect(hook.stored()).toMatchObject({ width: 600, lsb: 39, rsb: 61 })
    hook.unmount()
  })

  it('reports a baseline read failure once and still closes the edit', async () => {
    const report = vi
      .spyOn(commands, 'reportError')
      .mockImplementation(() => undefined)
    const hook = setup({ failRead: true })
    act(() => {
      hook.result.current.onMetricKey(key('left'))
    })
    await act(async () => {
      await hook.result.current.finish()
    })
    expect(hook.transport.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'glyphs.outline',
      'doc.edit_end',
    ])
    expect(report).toHaveBeenCalledOnce()
    expect(hook.reload).toHaveBeenCalledOnce()
    hook.unmount()
  })

  it('closes its captured document without publishing old preview into a replacement', async () => {
    const reading = deferred()
    const hook = setup({ read: reading.promise })
    act(() => {
      hook.result.current.onMetricKey(key('left'))
    })
    await vi.waitFor(() =>
      expect(hook.transport.calls.at(-1)?.method).toBe('glyphs.outline'),
    )
    useEditorStore.setState({ activeGlyph: { ...hook.target, doc: 2, gid: 8 } })
    hook.unmount()
    const closing = hook.result.current.finish()
    reading.resolve()
    await closing
    expect(
      hook.transport.calls.filter((entry) => entry.method === 'doc.edit_end')[0]
        ?.params,
    ).toMatchObject({ doc: 1 })
    expect(hook.preview).not.toHaveBeenCalled()
    expect(hook.reload).not.toHaveBeenCalled()
  })
})
