import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { setTransport } from '../../api'
import { activeEditor } from '../../editor/activeEditor'
import {
  initialDocumentState,
  useDocumentStore,
} from '../../stores/documentStore'
import { initialEditorState, useEditorStore } from '../../stores/editorStore'
import { initialUiState, useUiStore } from '../../stores/uiStore'
import {
  makeCell,
  makeContour,
  makeFontInfoView,
  makeGlyph,
  makeGlyphDetail,
  makeLayer,
  makeLayerOutline,
  makePoint,
} from '../../test/fontFixtures'
import {
  createMockTransport,
  makeSummary,
  noChange,
} from '../../test/mockTransport'
import { TextView } from './TextView'

const layer = makeLayer({
  width: 600,
  contours: [
    makeContour([
      makePoint(40, 0),
      makePoint(560, 0),
      makePoint(560, 700),
      makePoint(40, 700),
    ]),
  ],
})
const names = ['A', 'B', 'C']
const summary = makeSummary()
function cell(gid: number) {
  return makeCell({
    gid,
    name: names[gid - 1] ?? 'A',
    width: 600,
    path: 'M40 0L560 0L560 700L40 700Z',
  })
}
function transportFor(detailGate?: Promise<void>) {
  return createMockTransport({
    'glyphs.detail': async ({ gid }) => {
      await detailGate
      return makeGlyphDetail(
        makeGlyph({
          name: names[gid - 1] ?? 'A',
          unicodes: [64 + gid],
          layers: [layer],
        }),
      )
    },
    'glyphs.cell': ({ gid }) => cell(gid),
    'glyphs.search': ({ query }) =>
      names.includes(query) ? [cell(names.indexOf(query) + 1)] : [],
    'glyphs.outline': () => makeLayerOutline(layer),
    'doc.font_info': () => makeFontInfoView(),
    'doc.edit_begin': () => 5,
    'doc.edit_apply': () => noChange([3]),
    'doc.edit_end': () => null,
    'text.shape': ({ text }) => ({
      glyphs: Array.from(text).map((name, cluster) => ({
        gid: names.indexOf(name),
        cluster,
        x_advance: 600,
        y_advance: 0,
        x_offset: 0,
        y_offset: 0,
      })),
      names: Array.from(text),
      units_per_em: 1000,
      size: null,
      stale: false,
      warnings: [],
    }),
  })
}
function Harness() {
  const tab = useUiStore((state) =>
    state.tabs.find((tab) => tab.id === state.activeTab),
  )
  return tab ? <TextView summary={summary} tab={tab} /> : null
}
beforeEach(() => {
  localStorage.clear()
  useUiStore.setState(initialUiState())
  useDocumentStore.setState(initialDocumentState())
  useEditorStore.setState(initialEditorState())
  useUiStore.getState().openTab({ kind: 'glyph', doc: 1, gid: 1 })
})
afterEach(() => {
  setTransport(undefined)
  vi.restoreAllMocks()
})

describe('TextView integration', () => {
  it('migrates a legacy tab and retains the controller during a third-glyph local edit', async () => {
    const transport = transportFor()
    setTransport(transport)
    render(<Harness />)
    const canvas = await screen.findByRole('img', { name: /^Glyph A,/ })
    await waitFor(() =>
      expect(screen.getByRole('textbox', { name: 'Text' })).toHaveValue('A'),
    )
    const editor = activeEditor()!
    const id = useUiStore.getState().activeTab
    fireEvent.change(screen.getByRole('textbox', { name: 'Text' }), {
      target: { value: 'ABC' },
    })
    await waitFor(() => expect(globalThis.__fwTextRun?.glyphs).toHaveLength(3))
    const viewport = editor.viewportForTest
    act(() => {
      viewport.setSize(1000, 500, 1)
      viewport.zoomTo(0.5)
    })
    const target = viewport.toScreen({ x: 1300, y: 100 })
    fireEvent.pointerDown(canvas, {
      clientX: target.x,
      clientY: target.y,
      button: 0,
    })
    await waitFor(() =>
      expect(useEditorStore.getState().activeGlyph?.gid).toBe(3),
    )
    expect(activeEditor()).toBe(editor)
    expect(useUiStore.getState().activeTab).toBe(id)
    expect(useUiStore.getState().tabs[0]?.label).toBe('ABC')
    await waitFor(() =>
      expect(screen.getByRole('region', { name: 'Info box' })).toBeVisible(),
    )
    await waitFor(() =>
      expect(useEditorStore.getState().activeGlyph?.outline).not.toBeNull(),
    )
    const local = viewport.toScreen({ x: 40, y: 0 })
    act(() => {
      editor.pointerDown({
        ...local,
        button: 0,
        modifiers: { alt: false, shift: false, mod: false },
      })
      editor.pointerUp({
        ...local,
        button: 0,
        modifiers: { alt: false, shift: false, mod: false },
      })
    })
    expect(editor.currentSelection.size).toBe(1)
    fireEvent.keyDown(canvas, {
      key: 'ArrowRight',
      ctrlKey: true,
      altKey: true,
    })
    expect(
      transport.calls.filter((entry) => entry.method === 'doc.edit_apply'),
    ).toHaveLength(0)
    fireEvent.keyDown(canvas, { key: 'ArrowRight', shiftKey: true })
    await act(async () => {
      await editor.endNudge()
    })
    const moves = transport.calls.filter(
      (entry) => entry.method === 'doc.edit_apply',
    )
    expect(JSON.stringify(moves)).toContain('MovePoints')
    expect(moves[0]?.params).toMatchObject({
      command: {
        MovePoints: {
          glyph: 'C',
          points: [{ contour: 0, point: 0 }],
          delta: [10, 0],
        },
      },
    })
    fireEvent.keyDown(canvas, { key: 'Escape' })
    expect(useUiStore.getState().tabs[0]?.textState?.mode).toBe('edit')
    fireEvent.keyDown(canvas, { key: 'Escape' })
    await waitFor(() =>
      expect(useUiStore.getState().tabs[0]?.textState?.mode).toBe('text'),
    )
  })

  it('does not flush an unrelated editor from an old mounted view callback', async () => {
    setTransport(transportFor())
    const original = useUiStore.getState().tabs[0]!
    render(<TextView summary={summary} tab={original} />)
    await screen.findByRole('img', { name: /^Glyph A,/ })
    const prepare = vi.spyOn(activeEditor()!, 'prepareTransition')
    act(() => {
      useUiStore.getState().openTab({ kind: 'glyph', doc: 2, gid: 20 })
    })
    fireEvent.change(screen.getByRole('textbox', { name: 'Text' }), {
      target: { value: 'late' },
    })
    await act(async () => undefined)
    expect(prepare).not.toHaveBeenCalled()
  })

  it('does not overwrite early typing when initial glyph metadata arrives', async () => {
    let release: (() => void) | undefined
    setTransport(
      transportFor(
        new Promise<void>((resolve) => {
          release = resolve
        }),
      ),
    )
    render(<Harness />)
    const input = screen.getByRole('textbox', { name: 'Text' })
    fireEvent.change(input, { target: { value: 'ABC' } })
    await waitFor(() =>
      expect(useUiStore.getState().tabs[0]?.textState?.text).toBe('ABC'),
    )
    await act(async () => release?.())
    await screen.findByRole('img', { name: /^Glyph A,/ })
    expect(input).toHaveValue('ABC')
  })

  it('keeps typed text visible while a tool transition is delayed', async () => {
    setTransport(transportFor())
    render(<Harness />)
    await screen.findByRole('img', { name: /^Glyph A,/ })
    let release: (() => void) | undefined
    vi.spyOn(activeEditor()!, 'prepareTransition').mockReturnValue(
      new Promise<void>((resolve) => {
        release = resolve
      }),
    )
    const input = screen.getByRole('textbox', { name: 'Text' })
    fireEvent.change(input, { target: { value: 'A' } })
    fireEvent.change(input, { target: { value: 'AB' } })
    fireEvent.change(input, { target: { value: 'ABC' } })
    expect(input).toHaveValue('ABC')
    await act(async () => release?.())
    await waitFor(() =>
      expect(useUiStore.getState().tabs[0]?.textState?.text).toBe('ABC'),
    )
  })
})
