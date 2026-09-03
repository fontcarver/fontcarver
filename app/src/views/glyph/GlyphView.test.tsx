import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { setTransport, type GlyphDetailParams } from '../../api'
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
  makePoint,
} from '../../test/fontFixtures'
import { createMockTransport, makeSummary } from '../../test/mockTransport'

import { GlyphView } from './GlyphView'

const glyph = makeGlyph({
  name: 'aacute',
  unicodes: [0xe1],
  layers: [
    makeLayer({
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
    }),
  ],
})

function transportFor() {
  return createMockTransport({
    'glyphs.detail': () => makeGlyphDetail(glyph),
    'glyphs.cell': () => makeCell({ gid: 42, name: 'aacute', width: 600 }),
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

afterEach(() => setTransport(undefined))

describe('GlyphView', () => {
  it('requests the glyph once and shows its metrics in the readout', async () => {
    const transport = transportFor()
    setTransport(transport)

    render(<GlyphView summary={makeSummary()} gid={42} />)

    expect(await screen.findByText('aacute')).toBeInTheDocument()
    expect(screen.getByText('U+00E1')).toBeInTheDocument()
    // Advance 600 with bounds 40 … 560 gives 40 units on both sides.
    expect(screen.getByText('600')).toBeInTheDocument()
    expect(screen.getAllByText('40')).toHaveLength(2)

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

  it('reports a glyph that cannot be loaded', async () => {
    setTransport(
      createMockTransport({
        'glyphs.detail': () => {
          throw new Error('glyph handle 42 does not exist')
        },
        'glyphs.cell': () => makeCell(),
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
