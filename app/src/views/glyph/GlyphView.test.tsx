import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { setTransport, type GlyphDetailParams } from '../../api'
import { Viewport } from '../../canvas/Viewport'
import { activeEditor } from '../../editor/activeEditor'
import {
  initialDocumentState,
  useDocumentStore,
} from '../../stores/documentStore'
import { initialEditorState, useEditorStore } from '../../stores/editorStore'
import { forgetFontInfo } from '../../stores/fontInfoCache'
import { initialUiState, useUiStore } from '../../stores/uiStore'
import { resetOrders } from '../font/glyphOrder'
import {
  makeCell,
  makeContour,
  makeFontInfoView,
  makeGlyph,
  makeGlyphDetail,
  makeLayer,
  makeLayerOutline,
  makeMaster,
  makePoint,
} from '../../test/fontFixtures'
import {
  createMockTransport,
  makeSummary,
  noChange,
} from '../../test/mockTransport'

import { GlyphView } from './GlyphView'
import { commitField } from './commitField'

const layer = makeLayer({
  width: 600,
  contours: [
    makeContour([
      makePoint(40, 0),
      makePoint(560, 0),
      makePoint(560, 700, 'Line', true),
      makePoint(40, 700),
    ]),
  ],
  anchors: [{ name: 'top', x: 300, y: 700 }],
})

const glyph = makeGlyph({
  name: 'aacute',
  unicodes: [0xe1],
  layers: [layer],
})

function transportFor() {
  return createMockTransport({
    'glyphs.detail': () => makeGlyphDetail(glyph),
    'glyphs.cell': () => makeCell({ gid: 42, name: 'aacute', width: 600 }),
    'glyphs.outline': () => makeLayerOutline(layer),
    'doc.font_info': () => makeFontInfoView(),
  })
}

beforeEach(() => {
  localStorage.clear()
  useUiStore.setState(initialUiState())
  useDocumentStore.setState(initialDocumentState())
  useEditorStore.setState(initialEditorState())
  resetOrders()
})

afterEach(() => {
  setTransport(undefined)
  vi.restoreAllMocks()
})

describe('GlyphView', () => {
  it('requests the glyph once and shows its metrics in the readout', async () => {
    const transport = transportFor()
    setTransport(transport)

    render(<GlyphView summary={makeSummary()} gid={42} />)

    // The Info box needs both the glyph detail and the editable outline, so
    // two mocked round trips stand between the render and the name; the
    // default findBy timeout covers them with room to spare (~230 ms here).
    expect(await screen.findByText('aacute')).toBeInTheDocument()
    expect(screen.getByText('U+00E1')).toBeInTheDocument()
    // Advance 600 with bounds 40 … 560 gives 40 units on both sides.
    expect(screen.getByLabelText('advance')).toHaveValue('600')
    expect(screen.getByLabelText('LSB')).toHaveValue('40')
    expect(screen.getByLabelText('RSB')).toHaveValue('40')

    const details = transport.calls.filter(
      (entry) => entry.method === 'glyphs.detail',
    )
    expect(details).toHaveLength(1)
    expect(details[0]?.params).toEqual<GlyphDetailParams>({ doc: 1, gid: 42 })
  })

  it('exposes the canvas with an accessible name', async () => {
    setTransport(transportFor())
    render(<GlyphView summary={makeSummary()} gid={42} />)

    expect(
      await screen.findByRole('img', { name: 'Glyph aacute, U+00E1' }),
    ).toBeInTheDocument()
  })

  it('publishes the glyph for the inspector and names the tab', async () => {
    setTransport(transportFor())
    useUiStore.getState().openTab({ kind: 'glyph', doc: 1, gid: 42 })

    render(<GlyphView summary={makeSummary()} gid={42} />)
    await screen.findByText('aacute')

    await waitFor(() =>
      expect(useEditorStore.getState().activeGlyph?.gid).toBe(42),
    )
    expect(useEditorStore.getState().activeGlyph?.scene.anchors).toHaveLength(1)
    expect(useUiStore.getState().tabs[0]?.label).toBe('aacute')
  })

  it('toggles an overlay from the toolbar', async () => {
    const user = userEvent.setup()
    setTransport(transportFor())
    render(<GlyphView summary={makeSummary()} gid={42} />)
    await screen.findByText('aacute')

    expect(useEditorStore.getState().overlays.filled).toBe(false)
    await user.click(screen.getByRole('button', { name: 'Fill' }))
    expect(useEditorStore.getState().overlays.filled).toBe(true)
  })

  it('focuses the canvas on opening and on click so Space toggles fill', async () => {
    const user = userEvent.setup()
    setTransport(transportFor())
    useUiStore.getState().openTab({ kind: 'glyph', doc: 1, gid: 42 })

    render(<GlyphView summary={makeSummary()} gid={42} />)
    const canvas = await screen.findByRole('img', {
      name: 'Glyph aacute, U+00E1',
    })

    await waitFor(() => expect(canvas).toHaveFocus())
    canvas.blur()
    expect(canvas).not.toHaveFocus()
    await user.click(canvas)
    expect(canvas).toHaveFocus()

    await user.keyboard(' ')
    expect(useEditorStore.getState().overlays.filled).toBe(true)
  })

  it('Space-drags the newly focused canvas without toggling fill', async () => {
    const pan = vi.spyOn(Viewport.prototype, 'pan')
    setTransport(transportFor())
    useUiStore.getState().openTab({ kind: 'glyph', doc: 1, gid: 42 })

    render(<GlyphView summary={makeSummary()} gid={42} />)
    const canvas = await screen.findByRole('img', {
      name: 'Glyph aacute, U+00E1',
    })

    fireEvent.keyDown(canvas, { key: ' ', code: 'Space' })
    const pointer = (
      type: string,
      clientX: number,
      clientY: number,
    ): MouseEvent => {
      const event = new MouseEvent(type, {
        bubbles: true,
        button: 0,
        clientX,
        clientY,
      })
      Object.defineProperty(event, 'pointerId', { value: 7 })
      return event
    }
    fireEvent(canvas, pointer('pointerdown', 100, 120))
    fireEvent(canvas, pointer('pointermove', 125, 110))
    fireEvent(canvas, pointer('pointerup', 125, 110))
    fireEvent.keyUp(canvas, { key: ' ', code: 'Space' })

    expect(pan).toHaveBeenCalledWith(25, -10)
    expect(useEditorStore.getState().overlays.filled).toBe(false)
  })

  it('cancels a metrics gesture before settling Escape from the canvas', async () => {
    let release = () => undefined as void
    const until = new Promise<void>((resolve) => {
      release = resolve
    })
    const transport = createMockTransport({
      'glyphs.detail': () => makeGlyphDetail(glyph),
      'glyphs.cell': () => makeCell({ gid: 42, name: 'aacute', width: 600 }),
      'glyphs.outline': () => makeLayerOutline(layer),
      'doc.font_info': () => makeFontInfoView(),
      'doc.edit_begin': () => 5,
      'doc.edit_apply': () => noChange([42]),
      'doc.edit_end': async () => {
        await until
        return null
      },
      'doc.undo': () => noChange([42]),
      'doc.apply': () => noChange([42]),
    })
    setTransport(transport)
    render(<GlyphView summary={makeSummary()} gid={42} />)
    const canvas = await screen.findByRole('img', {
      name: 'Glyph aacute, U+00E1',
    })
    await waitFor(() => expect(activeEditor()).not.toBeNull())
    await waitFor(() =>
      expect(useEditorStore.getState().activeGlyph?.outline).not.toBeNull(),
    )
    const viewport = activeEditor()?.viewportForTest
    if (!viewport) throw new Error('editor did not mount')
    viewport.setSize(400, 400, 1)
    viewport.scale = 1
    viewport.offset = { x: 0, y: 300 }

    fireEvent(
      canvas,
      new PointerEvent('pointerdown', {
        bubbles: true,
        clientX: 40,
        clientY: 200,
        button: 0,
      }),
    )
    fireEvent(
      canvas,
      new PointerEvent('pointermove', {
        bubbles: true,
        clientX: 50,
        clientY: 200,
        button: 0,
      }),
    )
    await waitFor(() =>
      expect(transport.calls.map((entry) => entry.method)).toContain(
        'doc.edit_apply',
      ),
    )
    fireEvent.keyDown(canvas, { key: 'Escape' })
    commitField(
      { doc: 1, glyph: 'aacute', layer: layer.id },
      {
        SetMetrics: {
          glyph: 'aacute',
          layer: layer.id,
          width: 620,
          lsb: null,
          rsb: null,
        },
      },
      'Set metrics',
      () => undefined,
    )
    await waitFor(() =>
      expect(transport.calls.map((entry) => entry.method)).toContain(
        'doc.edit_end',
      ),
    )
    expect(
      transport.calls
        .map((entry) => entry.method)
        .filter((method) => method.startsWith('doc.edit_')),
    ).toEqual(['doc.edit_begin', 'doc.edit_apply', 'doc.edit_end'])
    expect(transport.calls.some((entry) => entry.method === 'doc.apply')).toBe(
      false,
    )

    release()
    await waitFor(() =>
      expect(
        transport.calls
          .map((entry) => entry.method)
          .filter(
            (method) =>
              method.startsWith('doc.edit_') ||
              method === 'doc.undo' ||
              method === 'doc.apply',
          ),
      ).toEqual([
        'doc.edit_begin',
        'doc.edit_apply',
        'doc.edit_end',
        'doc.undo',
        'doc.apply',
      ]),
    )
  })

  it('places zoom and Fit before the wrappable overlay toggles', async () => {
    setTransport(transportFor())
    render(<GlyphView summary={makeSummary()} gid={42} />)
    await screen.findByText('aacute')

    const fit = screen.getByRole('button', { name: 'Fit Glyph in Window' })
    const fill = screen.getByRole('button', { name: 'Fill' })
    expect(
      fit.compareDocumentPosition(fill) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).not.toBe(0)
  })

  it("uses the selected layer's master and clears it on toolbar changes", async () => {
    useUiStore.getState().openTab({ kind: 'glyph', doc: 1, gid: 42 })
    const user = userEvent.setup()
    const bold = makeLayer({
      id: 'bold-layer',
      kind: { Master: 'm2' },
      name: 'Bold',
      width: 700,
    })
    const both = makeGlyph({ ...glyph, layers: [layer, bold] })
    const regularMaster = makeMaster({
      guides: [
        {
          x: 0,
          y: 500,
          angle: 0,
          name: 'regular-guide',
          locked: false,
          color: null,
          anchor: 'Origin' as const,
        },
      ],
    })
    const boldMaster = makeMaster({
      id: 'm2',
      name: 'Bold',
      guides: [
        {
          x: 0,
          y: 600,
          angle: 0,
          name: 'bold-guide',
          locked: false,
          color: null,
          anchor: 'Origin' as const,
        },
      ],
    })
    forgetFontInfo(1)
    const mock = createMockTransport({
      'glyphs.detail': () => makeGlyphDetail(both),
      'glyphs.cell': () => makeCell({ gid: 42, name: 'aacute' }),
      'glyphs.outline': (params) =>
        params.layer === bold.id
          ? makeLayerOutline(bold, { master: 'm2' })
          : makeLayerOutline(layer, { master: 'm1' }),
      'doc.font_info': () =>
        makeFontInfoView({ masters: [regularMaster, boldMaster] }),
    })
    setTransport(mock)
    render(
      <GlyphView
        summary={makeSummary({
          masters: [
            { id: 'm1', name: 'Regular' },
            { id: 'm2', name: 'Bold' },
          ],
        })}
        gid={42}
      />,
    )
    await screen.findByText('aacute')

    // LayerList performs these two store writes from one row click.
    act(() => {
      useEditorStore.getState().setEditedLayer(bold.id)
      useDocumentStore.getState().setMaster(1, 'm2')
    })
    await waitFor(() => {
      const active = useEditorStore.getState().activeGlyph
      expect(active?.outline?.layer).toBe(bold.id)
      expect(active?.master).toBe('m2')
      expect(active?.masterGuides[0]?.name).toBe('bold-guide')
    })

    await user.selectOptions(
      screen.getByRole('combobox', { name: 'Master' }),
      'm1',
    )
    expect(useEditorStore.getState().editedLayer).toBeNull()
    await waitFor(() => {
      const active = useEditorStore.getState().activeGlyph
      expect(active?.outline?.layer).toBe(layer.id)
      expect(active?.master).toBe('m1')
      expect(active?.masterGuides[0]?.name).toBe('regular-guide')
    })
  })

  it('reports a glyph that cannot be loaded', async () => {
    setTransport(
      createMockTransport({
        'glyphs.detail': () => {
          throw new Error('glyph handle 42 does not exist')
        },
        'glyphs.cell': () => makeCell(),
        'glyphs.outline': () => makeLayerOutline(layer),
        'doc.font_info': () => makeFontInfoView(),
      }),
    )
    render(<GlyphView summary={makeSummary()} gid={42} />)

    expect(
      await screen.findByText('glyph handle 42 does not exist'),
    ).toBeInTheDocument()
  })

  it('says so when the tab carries no glyph', () => {
    setTransport(transportFor())
    render(<GlyphView summary={makeSummary()} gid={undefined} />)
    expect(screen.getByText('No glyph selected.')).toBeInTheDocument()
  })
})
