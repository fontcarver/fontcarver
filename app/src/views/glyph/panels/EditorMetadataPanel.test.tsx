import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { setTransport } from '../../../api'
import { initialEditorState, useEditorStore } from '../../../stores/editorStore'
import { useEditorMetadataStore } from '../../../editor/editorMetadataStore'
import { selectionOf } from '../../../editor/selection'
import {
  makeCell,
  makeContour,
  makeGlyph,
  makeGlyphDetail,
  makeLayer,
  makeLayerOutline,
  makeMaster,
  makePoint,
} from '../../../test/fontFixtures'
import { createMockTransport, noChange } from '../../../test/mockTransport'
import { buildScene } from '../glyphScene'
import { EditorMetadataPanel } from './EditorMetadataPanel'

const note = {
  id: 'a-note',
  kind: 'Text',
  x: 0,
  y: 0,
  end_x: 0,
  end_y: 0,
  text: 'Review stem',
}
function setup(locked = false) {
  const layer = makeLayer({
    contours: [makeContour([makePoint(0, 0), makePoint(100, 0)])],
    user_data: { 'com.fontcarver.editor': { annotations: [note] } },
  })
  const glyph = makeGlyph({ layers: [layer], locked })
  const scene = buildScene({
    detail: makeGlyphDetail(glyph),
    cell: makeCell(),
    master: makeMaster(),
    masterId: 'm1',
    unitsPerEm: 1000,
  })
  const transport = createMockTransport({ 'doc.apply': () => noChange([42]) })
  setTransport(transport)
  useEditorStore.setState({
    ...initialEditorState(),
    selection: selectionOf([{ kind: 'node', contour: 0, point: 0 }]),
  })
  render(
    <EditorMetadataPanel
      target={{ doc: 1, glyph: 'a', layer: layer.id }}
      gid={42}
      glyph={glyph}
      outline={makeLayerOutline(layer)}
      scene={scene}
      cell={makeCell()}
      master="m1"
      masterGuides={[]}
      reload={vi.fn()}
    />,
  )
  return transport
}

beforeEach(() => useEditorMetadataStore.getState().clearFocus())
afterEach(() => setTransport(undefined))

describe('editor metadata inspector', () => {
  it('focuses selected paths without a document mutation and sends lock commands', async () => {
    const transport = setup()
    fireEvent.click(
      screen.getByRole('button', { name: 'Focus selected paths' }),
    )
    expect(useEditorMetadataStore.getState().focus?.contours).toEqual(
      new Set([0]),
    )
    expect(transport.calls).toHaveLength(0)
    fireEvent.click(screen.getByRole('button', { name: 'Lock selected paths' }))
    await waitFor(() =>
      expect(transport.calls.some((call) => call.method === 'doc.apply')).toBe(
        true,
      ),
    )
    expect(
      transport.calls.find((call) => call.method === 'doc.apply')?.params,
    ).toMatchObject({
      command: {
        EditorMetadata: {
          PathLocks: {
            glyph: 'a',
            layer: 'layer-1',
            contours: [0],
            locked: true,
          },
        },
      },
    })
  })

  it('edits and removes the selected layer annotation', async () => {
    const transport = setup()
    fireEvent.change(screen.getByRole('combobox', { name: 'Annotation' }), {
      target: { value: note.id },
    })
    fireEvent.change(screen.getByRole('textbox', { name: 'Text' }), {
      target: { value: 'Recheck curve' },
    })
    fireEvent.blur(screen.getByRole('textbox', { name: 'Text' }))
    await waitFor(() => expect(transport.calls).toHaveLength(1))
    expect(transport.calls[0]?.params).toMatchObject({
      command: {
        EditorMetadata: {
          Annotations: {
            annotations: [{ ...note, text: 'Recheck curve' }],
          },
        },
      },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Remove annotation' }))
    await waitFor(() => expect(transport.calls).toHaveLength(2))
    expect(transport.calls[1]?.params).toMatchObject({
      command: { EditorMetadata: { Annotations: { annotations: [] } } },
    })
  })

  it('disables mutation controls for a locked glyph', () => {
    setup(true)
    expect(
      screen.getByRole('button', { name: 'Lock selected paths' }),
    ).toBeDisabled()
    expect(
      screen.getByRole('button', { name: 'Draw annotation' }),
    ).toBeDisabled()
  })
})
