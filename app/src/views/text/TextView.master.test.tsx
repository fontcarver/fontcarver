import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
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
import { TextView } from '../../views/text/TextView'

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
const summary = makeSummary({
  masters: [
    { id: 'm1', name: 'Regular' },
    { id: 'm2', name: 'Bold' },
  ],
})
function cell(gid: number) {
  return makeCell({
    gid,
    name: names[gid - 1] ?? 'A',
    width: 600,
    path: 'M40 0L560 0L560 700L40 700Z',
  })
}
function transportFor(applyGate?: Promise<void>) {
  return createMockTransport({
    'glyphs.detail': ({ gid }) => {
      return makeGlyphDetail(
        makeGlyph({
          name: names[gid - 1] ?? 'A',
          unicodes: [64 + gid],
          layers: [
            layer,
            { ...layer, id: 'bold-layer', kind: { Master: 'm2' } },
          ],
        }),
      )
    },
    'glyphs.cell': ({ gid }) => cell(gid),
    'glyphs.search': ({ query }) =>
      names.includes(query) ? [cell(names.indexOf(query) + 1)] : [],
    'glyphs.outline': ({ layer: id }) =>
      id === 'bold-layer'
        ? makeLayerOutline(
            { ...layer, id: 'bold-layer', kind: { Master: 'm2' } },
            { master: 'm2' },
          )
        : makeLayerOutline(layer),
    'doc.font_info': () => makeFontInfoView(),
    'doc.edit_begin': () => 5,
    'doc.edit_apply': () => noChange([3]),
    'doc.edit_end': () => null,
    'doc.apply': async () => {
      await applyGate
      return noChange([1])
    },
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

it('commits a live pen path to the old layer before the inner toolbar changes master', async () => {
  let release: (() => void) | undefined
  const transport = transportFor(
    new Promise<void>((resolve) => {
      release = resolve
    }),
  )
  setTransport(transport)
  render(<Harness />)
  await screen.findByRole('img', { name: /^Glyph A,/ })
  await waitFor(() =>
    expect(useEditorStore.getState().activeGlyph?.outline).not.toBeNull(),
  )
  const editor = activeEditor()!
  act(() => {
    useEditorStore.getState().setTool('pen')
  })
  act(() => {
    const vp = editor.viewportForTest
    vp.setSize(1000, 500, 1)
    vp.zoomTo(0.5)
    for (const at of [
      { x: 100, y: 100 },
      { x: 250, y: 200 },
    ]) {
      const p = {
        ...vp.toScreen(at),
        button: 0,
        modifiers: { alt: false, shift: false, mod: false },
      }
      editor.pointerDown(p)
      editor.pointerUp(p)
    }
  })
  expect(editor.claimsKey('Enter')).toBe(true)
  // Exercise the inner glyph toolbar, alongside the TextBar master selector.
  const selects = screen.getAllByRole('combobox', { name: 'Master' })
  fireEvent.change(selects.at(-1)!, { target: { value: 'm2' } })
  await waitFor(() =>
    expect(
      transport.calls.filter((c) => c.method === 'doc.apply'),
    ).toHaveLength(1),
  )
  expect(useDocumentStore.getState().byDoc[1]?.master).not.toBe('m2')
  expect(useEditorStore.getState().activeGlyph?.outline?.layer).toBe(layer.id)
  await act(async () => release?.())
  await waitFor(() =>
    expect(useUiStore.getState().tabs[0]?.textState?.master).toBe('m2'),
  )
  await waitFor(() =>
    expect(useEditorStore.getState().activeGlyph?.master).toBe('m2'),
  )
  await act(async () => {
    await editor.prepareTransition()
  })
  const applies = transport.calls.filter((c) => c.method === 'doc.apply')
  expect(applies).toHaveLength(1)
  expect(applies[0]?.params).toMatchObject({
    doc: 1,
    command: {
      AddContour: {
        glyph: 'A',
        layer: layer.id,
        contour: {
          points: [
            { x: 100, y: 100 },
            { x: 250, y: 200 },
          ],
        },
      },
    },
  })
  expect(editor.claimsKey('Enter')).toBe(false)
})
