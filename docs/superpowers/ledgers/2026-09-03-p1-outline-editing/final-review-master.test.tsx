import { act, fireEvent, render, screen, waitFor } from '/home/chen/githubprojects/fontpixel/fontcutter/app/node_modules/@testing-library/react/dist/index.js'
import { afterEach, beforeEach, describe, expect, it, vi } from '/home/chen/githubprojects/fontpixel/fontcutter/app/node_modules/vitest/dist/index.js'
import { setTransport } from '/home/chen/githubprojects/fontpixel/fontcutter/app/src/api'
import { activeEditor } from '/home/chen/githubprojects/fontpixel/fontcutter/app/src/editor/activeEditor'
import {
  initialDocumentState,
  useDocumentStore,
} from '/home/chen/githubprojects/fontpixel/fontcutter/app/src/stores/documentStore'
import { initialEditorState, useEditorStore } from '/home/chen/githubprojects/fontpixel/fontcutter/app/src/stores/editorStore'
import { initialUiState, useUiStore } from '/home/chen/githubprojects/fontpixel/fontcutter/app/src/stores/uiStore'
import {
  makeCell,
  makeContour,
  makeFontInfoView,
  makeGlyph,
  makeGlyphDetail,
  makeLayer,
  makeLayerOutline,
  makePoint,
} from '/home/chen/githubprojects/fontpixel/fontcutter/app/src/test/fontFixtures'
import {
  createMockTransport,
  makeSummary,
  noChange,
} from '/home/chen/githubprojects/fontpixel/fontcutter/app/src/test/mockTransport'
import { TextView } from '/home/chen/githubprojects/fontpixel/fontcutter/app/src/views/text/TextView'

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
const summary = makeSummary({masters:[{id:'m1',name:'Regular'},{id:'m2',name:'Bold'}]})
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
          layers: [layer, {...layer,id:'bold-layer',kind:{Master:'m2'}}],
        }),
      )
    },
    'glyphs.cell': ({ gid }) => cell(gid),
    'glyphs.search': ({ query }) =>
      names.includes(query) ? [cell(names.indexOf(query) + 1)] : [],
    'glyphs.outline': ({layer: id}) => id==='bold-layer' ? makeLayerOutline({...layer,id:'bold-layer',kind:{Master:'m2'}},{master:'m2'}) : makeLayerOutline(layer),
    'doc.font_info': () => makeFontInfoView(),
    'doc.edit_begin': () => 5,
    'doc.edit_apply': () => noChange([3]),
    'doc.edit_end': () => null,
    'doc.apply': () => noChange([1]),
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


it('reproduction: GlyphToolbar master switch drops a live pen path in integrated TextView', async () => {
 const transport=transportFor(); setTransport(transport); render(<Harness />)
 await screen.findByRole('img', {name: /^Glyph A,/})
 await waitFor(()=>expect(useEditorStore.getState().activeGlyph?.outline).not.toBeNull())
 const editor=activeEditor()!
 act(()=>{useEditorStore.getState().setTool('pen')})
 act(()=>{
  const vp=editor.viewportForTest; vp.setSize(1000,500,1); vp.zoomTo(0.5)
  for (const at of [{x:100,y:100},{x:250,y:200}]) {
   const p={...vp.toScreen(at),button:0,modifiers:{alt:false,shift:false,mod:false}}
   editor.pointerDown(p);editor.pointerUp(p)
  }
 })
 expect(editor.claimsKey('Enter')).toBe(true)
 // TextBar plus glyph toolbar both have a Master combobox; the latter is ungated.
 const selects=screen.getAllByRole('combobox',{name:'Master'})
 fireEvent.change(selects.at(-1)!, {target:{value:'m2'}})
 await waitFor(()=>expect(useUiStore.getState().tabs[0]?.textState?.master).toBe('m2'))
 await waitFor(()=>expect(useEditorStore.getState().activeGlyph?.master).toBe('m2'))
 await act(async()=>{await editor.prepareTransition()})
 const applies=transport.calls.filter(c=>c.method==='doc.apply')
 console.log('MASTER SWITCH pen applies', JSON.stringify(applies), 'still claims Enter', editor.claimsKey('Enter'))
 expect(applies).toHaveLength(0)
 expect(editor.claimsKey('Enter')).toBe(false)
})
