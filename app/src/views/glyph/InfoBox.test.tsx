import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { setTransport, type ApplyParams } from '../../api'
import { selectionOf, type Selection } from '../../editor/selection'
import { setCommandErrorReporter } from '../../shell/commands'
import { initialEditorState, useEditorStore } from '../../stores/editorStore'
import { initialUiState, useUiStore } from '../../stores/uiStore'
import { initialTextState } from '../text/textState'
import {
  makeContour,
  makeLayer,
  makeLayerOutline,
  makePoint,
} from '../../test/fontFixtures'
import {
  createMockTransport,
  type MockTransport,
} from '../../test/mockTransport'

import { InfoBox } from './InfoBox'

const layer = makeLayer({
  width: 600,
  contours: [
    makeContour([
      makePoint(40, 0),
      makePoint(560, 0, 'Line'),
      makePoint(560, 700, 'Line'),
      makePoint(40, 700, 'Line'),
    ]),
  ],
  anchors: [{ name: 'top', x: 300, y: 700 }],
})

const outline = makeLayerOutline(layer)

const masterGuides = [
  {
    x: 0,
    y: 500,
    angle: 0,
    name: 'x-height',
    locked: false,
    color: null,
    anchor: 'Origin' as const,
  },
]

function transport(): MockTransport {
  const mock = createMockTransport({
    'doc.apply': () => ({
      glyphs: [42],
      font_info: false,
      masters: false,
      layout: false,
      bitmaps: false,
      math: false,
      color: false,
      feature_source: false,
      glyph_order: false,
      metrics: true,
      layers: false,
      guides: false,
    }),
  })
  setTransport(mock)
  return mock
}

function show(selection: Selection) {
  return render(
    <InfoBox
      target={{ doc: 1, glyph: 'a', layer: 'layer-1' }}
      outline={outline}
      selection={selection}
      master="m1"
      masterGuides={masterGuides}
      inkEdges={{ left: 0, right: 600 }}
      name="a"
      unicodes={[0x61]}
      onCommitted={() => undefined}
    />,
  )
}

function applied(mock: MockTransport): ApplyParams | undefined {
  return mock.calls.find((entry) => entry.method === 'doc.apply')?.params as
    ApplyParams | undefined
}

beforeEach(() => {
  useEditorStore.setState(initialEditorState())
  useUiStore.setState(initialUiState())
})
afterEach(() => {
  setTransport(undefined)
  setCommandErrorReporter(() => undefined)
})

describe('InfoBox', () => {
  it('commits a signed smart-corner radius to the source node', async () => {
    const user = userEvent.setup()
    const mock = transport()
    show(selectionOf([{ kind: 'node', contour: 0, point: 1 }]))
    const field = screen.getByLabelText('Corner radius')
    await user.clear(field)
    await user.type(field, '-20{Enter}')
    await waitFor(() =>
      expect(applied(mock)?.command).toEqual({
        SetSmartCorner: {
          glyph: 'a',
          layer: 'layer-1',
          points: [{ contour: 0, point: 1 }],
          radius: -20,
        },
      }),
    )
  })

  it('sets a shared radius on selected nodes without including anchors', async () => {
    const user = userEvent.setup()
    const mock = transport()
    show(
      selectionOf([
        { kind: 'node', contour: 0, point: 1 },
        { kind: 'node', contour: 0, point: 2 },
        { kind: 'anchor', index: 0 },
      ]),
    )
    const field = screen.getByLabelText('Corner radius')
    await user.clear(field)
    await user.type(field, '15{Enter}')
    await waitFor(() =>
      expect(applied(mock)?.command).toEqual({
        SetSmartCorner: {
          glyph: 'a',
          layer: 'layer-1',
          points: [
            { contour: 0, point: 1 },
            { contour: 0, point: 2 },
          ],
          radius: 15,
        },
      }),
    )
  })

  it('commits a node coordinate as one SetPointPosition', async () => {
    const user = userEvent.setup()
    const mock = transport()
    show(selectionOf([{ kind: 'node', contour: 0, point: 1 }]))

    const field = screen.getByLabelText('X')
    expect(field).toHaveValue('560')
    await user.clear(field)
    await user.type(field, '600{Enter}')

    expect(applied(mock)).toEqual({
      doc: 1,
      label: 'Set position',
      command: {
        SetPointPosition: {
          glyph: 'a',
          layer: 'layer-1',
          point: { contour: 0, point: 1 },
          x: 600,
          y: 0,
        },
      },
    })
  })

  it('accepts a relative expression in a field', async () => {
    const user = userEvent.setup()
    const mock = transport()
    show(selectionOf([{ kind: 'node', contour: 0, point: 1 }]))

    const field = screen.getByLabelText('Y')
    await user.clear(field)
    await user.type(field, '+40{Enter}')

    expect(applied(mock)?.command).toMatchObject({
      SetPointPosition: { x: 560, y: 40 },
    })
  })

  it('rejects nonsense without touching the outline', async () => {
    const user = userEvent.setup()
    const mock = transport()
    show(selectionOf([{ kind: 'node', contour: 0, point: 1 }]))

    const field = screen.getByLabelText('X')
    await user.clear(field)
    await user.type(field, 'wide{Enter}')

    expect(applied(mock)).toBeUndefined()
    expect(field).toHaveValue('560')
  })

  it('shows the smooth toggle for a node and commits SetSmooth', async () => {
    const user = userEvent.setup()
    const mock = transport()
    show(selectionOf([{ kind: 'node', contour: 0, point: 1 }]))

    await user.click(screen.getByRole('button', { name: 'Corner' }))
    expect(applied(mock)?.command).toMatchObject({
      SetSmooth: { smooth: true },
    })
  })

  it('commits a metric with nothing selected', async () => {
    const user = userEvent.setup()
    const mock = transport()
    show(new Map())

    const field = screen.getByLabelText('advance')
    await user.clear(field)
    await user.type(field, '640{Enter}')

    expect(applied(mock)?.command).toEqual({
      SetMetrics: {
        glyph: 'a',
        layer: 'layer-1',
        width: 640,
        lsb: null,
        rsb: null,
      },
    })
  })

  it('scales a multi-selection about the chosen origin', async () => {
    const user = userEvent.setup()
    const mock = transport()
    show(
      selectionOf([
        { kind: 'node', contour: 0, point: 0 },
        { kind: 'node', contour: 0, point: 1 },
        { kind: 'node', contour: 0, point: 2 },
      ]),
    )

    expect(screen.getByText('3 of 4 points')).toBeInTheDocument()
    const width = screen.getByLabelText('W')
    expect(width).toHaveValue('520')
    await user.clear(width)
    await user.type(width, '260{Enter}')

    const command = applied(mock)?.command
    expect(command).toMatchObject({
      TransformSelection: {
        glyph: 'a',
        layer: 'layer-1',
        points: [
          { contour: 0, point: 0 },
          { contour: 0, point: 1 },
          { contour: 0, point: 2 },
        ],
      },
    })
    // Halving the width about the centre keeps the centre where it was.
    expect(
      command && 'TransformSelection' in command
        ? command.TransformSelection.affine
        : null,
    ).toEqual([0.5, 0, 0, 1, 150, 0])
  })

  it('commits an anchor position', async () => {
    const user = userEvent.setup()
    const mock = transport()
    show(selectionOf([{ kind: 'anchor', index: 0 }]))

    const field = screen.getByLabelText('Y')
    await user.clear(field)
    await user.type(field, '720{Enter}')

    expect(applied(mock)?.command).toEqual({
      SetAnchor: {
        glyph: 'a',
        layer: 'layer-1',
        index: 0,
        anchor: { name: 'top', x: 300, y: 720 },
      },
    })
  })

  it('reports a refused commit and leaves the field usable', async () => {
    const user = userEvent.setup()
    const errors: unknown[] = []
    setCommandErrorReporter((error) => errors.push(error))
    setTransport(
      createMockTransport({
        'doc.apply': () => {
          throw new Error('point index 1 does not exist')
        },
      }),
    )
    show(selectionOf([{ kind: 'node', contour: 0, point: 1 }]))

    const field = screen.getByLabelText('X')
    await user.clear(field)
    await user.type(field, '600{Enter}')

    await waitFor(() =>
      expect(String(errors[0])).toContain('point index 1 does not exist'),
    )
    expect(field).toBeEnabled()
  })

  it('edits a master guide, which the canvas can select too', async () => {
    const user = userEvent.setup()
    const mock = transport()
    show(selectionOf([{ kind: 'guide', index: 0, scope: 'master' }]))

    const field = screen.getByLabelText('Y')
    expect(field).toHaveValue('500')
    await user.clear(field)
    await user.type(field, '520{Enter}')

    expect(applied(mock)?.command).toEqual({
      SetGuide: {
        glyph: null,
        layer: null,
        master: 'm1',
        index: 0,
        guide: {
          x: 0,
          y: 520,
          angle: 0,
          name: 'x-height',
          locked: false,
          color: null,
          anchor: 'Origin' as const,
        },
      },
    })
  })

  it('anchors a guide to a sidebearing without moving it', async () => {
    const user = userEvent.setup()
    const mock = transport()
    show(selectionOf([{ kind: 'guide', index: 0, scope: 'master' }]))

    // The stored guide is absolute at x 0; the ink starts at x 0 and ends at
    // 600, so anchoring to the right sidebearing rebases the offset to −600.
    await user.selectOptions(
      screen.getByLabelText('Anchored to'),
      'Right sidebearing',
    )
    expect(applied(mock)?.command).toEqual({
      SetGuide: {
        glyph: null,
        layer: null,
        master: 'm1',
        index: 0,
        guide: {
          x: -600,
          y: 500,
          angle: 0,
          name: 'x-height',
          locked: false,
          color: null,
          anchor: 'Rsb' as const,
        },
      },
    })
  })

  it('includes a selected anchor in the multi-selection box', () => {
    transport()
    show(
      selectionOf([
        { kind: 'node', contour: 0, point: 0 },
        { kind: 'node', contour: 0, point: 1 },
        { kind: 'anchor', index: 0 },
      ]),
    )

    // The anchor sits at (300, 700), above the two nodes on the baseline, so
    // the box has to reach it — the transform is going to move it.
    expect(screen.getByLabelText('H')).toHaveValue('700')
  })

  it('collapses and expands from its header', async () => {
    const user = userEvent.setup()
    transport()
    show(new Map())

    expect(screen.getByLabelText('advance')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Collapse info box' }))
    expect(screen.queryByLabelText('advance')).not.toBeInTheDocument()
  })

  it('locks its metrics fields while the active text tab locks spacing', async () => {
    const user = userEvent.setup()
    const mock = transport()
    useUiStore.getState().openTab({
      kind: 'glyph',
      doc: 1,
      gid: 42,
      textState: { ...initialTextState('m1', 'AV'), kerningMode: 'locked' },
    })
    show(selectionOf([]))
    for (const label of ['advance', 'LSB', 'RSB'])
      expect(screen.getByLabelText(label)).toBeDisabled()
    // A queued commit must not slip through the disabled control either.
    await user.type(screen.getByLabelText('LSB'), '99{Enter}')
    expect(applied(mock)).toBeUndefined()
  })
})
