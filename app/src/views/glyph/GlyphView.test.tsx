import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { setTransport, type GlyphDetailParams } from '../../api'
import { Viewport } from '../../canvas/Viewport'
import {
  initialDocumentState,
  useDocumentStore,
} from '../../stores/documentStore'
import { initialEditorState, useEditorStore } from '../../stores/editorStore'
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
  makePoint,
} from '../../test/fontFixtures'
import { createMockTransport, makeSummary } from '../../test/mockTransport'

import { GlyphView } from './GlyphView'

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
    // two round trips stand between the render and the name.
    expect(
      await screen.findByText('aacute', {}, { timeout: 5000 }),
    ).toBeInTheDocument()
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
    await screen.findByText('aacute', {}, { timeout: 5000 })

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
    await screen.findByText('aacute', {}, { timeout: 5000 })

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

    expect(canvas).toHaveFocus()
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

  it('places zoom and Fit before the wrappable overlay toggles', async () => {
    setTransport(transportFor())
    render(<GlyphView summary={makeSummary()} gid={42} />)
    await screen.findByText('aacute', {}, { timeout: 5000 })

    const fit = screen.getByRole('button', { name: 'Fit Glyph in Window' })
    const fill = screen.getByRole('button', { name: 'Fill' })
    expect(
      fit.compareDocumentPosition(fill) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).not.toBe(0)
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
