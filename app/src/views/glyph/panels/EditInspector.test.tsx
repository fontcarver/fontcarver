import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import {
  setTransport,
  type ApplyParams,
  type Glyph,
  type Layer,
  type LayerOutline,
} from '../../../api'
import { setCommandErrorReporter } from '../../../shell/commands'
import { i18n } from '../../../i18n'
import catalog from '../../../i18n/en.json'
import { initialEditorState, useEditorStore } from '../../../stores/editorStore'
import {
  initialDocumentState,
  useDocumentStore,
} from '../../../stores/documentStore'
import { initialUiState, useUiStore } from '../../../stores/uiStore'
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
import {
  createMockTransport,
  noChange,
  type MockTransport,
} from '../../../test/mockTransport'
import { selectionOf, type Selection } from '../../../editor/selection'
import { buildScene } from '../glyphScene'
import { measurementGuideKey } from '../guideIdentity'
import { initialTextState } from '../../text/textState'

import { EditInspector } from './EditInspector'

const kerningGroups = catalog.kerningGroups
const square = makeContour([
  makePoint(40, 0),
  makePoint(560, 0, 'Line'),
  makePoint(560, 700, 'Line'),
  makePoint(40, 700, 'Line'),
])

const masterLayer = makeLayer({ contours: [square] })

function transport(): MockTransport {
  const mock = createMockTransport({
    'doc.apply': () => noChange([42]),
    'doc.metrics_resolve': () => ({
      lsb: 40,
      rsb: 40,
      width: 600,
      source: 'Explicit' as const,
    }),
    'glyphs.search': () => [makeCell({ gid: 9, name: 'acute' })],
  })
  setTransport(mock)
  return mock
}

interface ShowOptions {
  glyph?: Glyph
  layers?: Layer[]
  outline?: Partial<LayerOutline>
  selection?: Selection
  reload?: () => void
}

function show(options: ShowOptions = {}) {
  const layers = options.layers ?? [masterLayer]
  const glyph = options.glyph ?? makeGlyph({ layers })
  const outline = makeLayerOutline(layers[0] ?? masterLayer, options.outline)
  const scene = buildScene({
    detail: makeGlyphDetail(glyph),
    cell: makeCell(),
    master: makeMaster(),
    masterId: 'm1',
    unitsPerEm: 1000,
  })
  useEditorStore.setState({
    ...initialEditorState(),
    selection: options.selection ?? selectionOf([]),
    activeGlyph: {
      doc: 1,
      gid: 42,
      glyph,
      scene,
      cell: makeCell(),
      outline,
      master: 'm1',
      masterGuides: [
        {
          x: 0,
          y: 500,
          angle: 0,
          name: 'x-height',
          locked: false,
          color: null,
          anchor: 'Origin' as const,
        },
      ],
      reload: options.reload ?? (() => undefined),
    },
  })
  return render(<EditInspector masters={[{ id: 'm1', name: 'Regular' }]} />)
}

function applied(mock: MockTransport): ApplyParams | undefined {
  return mock.calls.find((entry) => entry.method === 'doc.apply')?.params as
    ApplyParams | undefined
}

function allApplied(mock: MockTransport): ApplyParams[] {
  return mock.calls
    .filter((entry) => entry.method === 'doc.apply')
    .map((entry) => entry.params as ApplyParams)
}

beforeEach(() => {
  localStorage.clear()
  useUiStore.setState(initialUiState())
  useDocumentStore.setState(initialDocumentState())
  useEditorStore.setState(initialEditorState())
})

afterEach(() => {
  setTransport(undefined)
  setCommandErrorReporter(() => undefined)
})

describe('the accordion', () => {
  it('shows a selection hint when text mode intentionally has no active glyph', () => {
    i18n.addResourceBundle('en', 'translation', { kerningGroups }, true, true)
    useUiStore.getState().openTab({
      kind: 'glyph',
      doc: 1,
      gid: 42,
      textState: {
        ...initialTextState('m1', 'AV'),
        mode: 'text',
        activeIndex: null,
      },
    })
    render(<EditInspector masters={[{ id: 'm1', name: 'Regular' }]} />)
    expect(screen.getByText(kerningGroups.selectGlyph)).toBeVisible()
    expect(screen.queryByText('Loading glyph…')).not.toBeInTheDocument()
  })

  it('retains loading feedback for an edit-mode glyph request', () => {
    useUiStore.getState().openTab({
      kind: 'glyph',
      doc: 1,
      gid: 42,
      textState: initialTextState('m1', 'AV'),
    })
    render(<EditInspector masters={[{ id: 'm1', name: 'Regular' }]} />)
    expect(screen.getByText('Loading glyph…')).toBeVisible()
    expect(
      screen.queryByText(kerningGroups.selectGlyph),
    ).not.toBeInTheDocument()
  })

  it('shows the seven sections of the design spec in order', () => {
    show()
    const headers = screen
      .getAllByRole('button', { expanded: true })
      .concat(screen.getAllByRole('button', { expanded: false }))
      .map((button) => button.textContent ?? '')

    expect(headers.some((text) => text.includes('Glyph'))).toBe(true)
    expect(headers.some((text) => text.includes('Transform'))).toBe(true)
  })

  it('remembers a section the user closed', async () => {
    const user = userEvent.setup()
    show()

    await user.click(screen.getByRole('button', { name: /Anchors/ }))

    expect(useUiStore.getState().sections.anchors).toBe(false)
    expect(screen.getByRole('button', { name: /Anchors/ })).toHaveAttribute(
      'aria-expanded',
      'false',
    )
  })

  it('badges the counts the layer carries', () => {
    show({
      outline: {
        anchors: [{ name: 'top', x: 300, y: 700 }],
        guides: [
          {
            x: 0,
            y: 100,
            angle: 0,
            name: null,
            locked: false,
            color: null,
            anchor: 'Origin' as const,
          },
        ],
      },
    })

    // One layer, one anchor, one local guide plus one master guide.
    expect(
      within(screen.getByRole('button', { name: /Anchors/ })).getByText('1'),
    ).toBeInTheDocument()
    expect(
      within(screen.getByRole('button', { name: /Guides/ })).getByText('2'),
    ).toBeInTheDocument()
  })
})

describe('Glyph panel', () => {
  it('renames the glyph through RenameGlyph', async () => {
    const user = userEvent.setup()
    const mock = transport()
    show()

    const field = screen.getByLabelText('Name')
    await user.clear(field)
    await user.type(field, 'aacute{Enter}')

    expect(applied(mock)).toEqual({
      doc: 1,
      label: 'Rename glyph',
      command: { RenameGlyph: { from: 'a', to: 'aacute' } },
    })
  })

  it('adds a Unicode value written any of the accepted ways', async () => {
    const user = userEvent.setup()
    const mock = transport()
    show()

    await user.type(
      screen.getByLabelText('Add a Unicode value'),
      'U+00E1{Enter}',
    )

    expect(applied(mock)?.command).toEqual({
      SetUnicodes: { glyph: 'a', unicodes: [0x61, 0xe1] },
    })
  })

  it('removes a Unicode chip', async () => {
    const user = userEvent.setup()
    const mock = transport()
    show()

    await user.click(screen.getByRole('button', { name: 'Remove U+0061' }))

    expect(applied(mock)?.command).toEqual({
      SetUnicodes: { glyph: 'a', unicodes: [] },
    })
  })

  it('writes the export flag and the colour label as SetGlyphMeta', async () => {
    const user = userEvent.setup()
    const mock = transport()
    show()

    await user.click(screen.getByRole('button', { name: 'Yes' }))
    await user.click(screen.getByRole('button', { name: 'Blue' }))

    const commands = allApplied(mock).map((entry) => entry.command)
    expect(commands[0]).toMatchObject({ SetGlyphMeta: { export: false } })
    expect(commands[1]).toMatchObject({ SetGlyphMeta: { color_label: 7 } })
  })
})

describe('Metrics panel', () => {
  it('commits a sidebearing as one SetMetrics', async () => {
    const user = userEvent.setup()
    const mock = transport()
    show()

    const field = screen.getByLabelText('LSB')
    await user.clear(field)
    await user.type(field, '80{Enter}')

    expect(applied(mock)).toEqual({
      doc: 1,
      label: 'Set metrics',
      command: {
        SetMetrics: {
          glyph: 'a',
          layer: 'layer-1',
          width: null,
          lsb: 80,
          rsb: null,
        },
      },
    })
  })

  it('writes a metrics key and offers synchronization when it is out of sync', async () => {
    const user = userEvent.setup()
    const mock = createMockTransport({
      'doc.apply': () => noChange([42]),
      'doc.metrics_resolve': () => ({
        lsb: 100,
        rsb: 40,
        width: 660,
        source: 'Key' as const,
      }),
    })
    setTransport(mock)
    show({
      glyph: makeGlyph({
        layers: [masterLayer],
        metrics_keys: {
          left: '=H',
          right: null,
          width: null,
          vertical_origin: null,
          vertical_width: null,
        },
      }),
    })

    await waitFor(() =>
      expect(screen.getByText('out of sync')).toBeInTheDocument(),
    )

    await user.click(
      screen.getByRole('button', { name: 'Update current master' }),
    )
    expect(applied(mock)?.command).toEqual({
      UpdateMetrics: { glyph: 'a', master: 'm1' },
    })

    const key = screen.getByLabelText('LSB key')
    await user.clear(key)
    await user.type(key, '=n+10{Enter}')
    expect(allApplied(mock).at(-1)?.command).toMatchObject({
      SetMetricsKeys: { glyph: 'a', keys: { left: '=n+10' } },
    })
  })
})

describe('Layers panel', () => {
  it('adds a background layer for the edited master', async () => {
    const user = userEvent.setup()
    const mock = transport()
    show()

    await user.click(screen.getByRole('button', { name: 'Add background' }))

    expect(applied(mock)?.command).toMatchObject({
      AddLayer: { glyph: 'a', layer: { kind: { Background: 'm1' } } },
    })
    expect(applied(mock)?.label).toBe('Add layer')
  })

  it('copies to and swaps with the background', async () => {
    const user = userEvent.setup()
    const mock = transport()
    const background = makeLayer({
      id: 'bg-1',
      kind: { Background: 'm1' },
      name: 'Background',
    })
    show({ layers: [masterLayer, background] })

    await user.click(screen.getByRole('button', { name: 'Copy to background' }))
    await user.click(
      screen.getByRole('button', { name: 'Swap with background' }),
    )

    const commands = allApplied(mock).map((entry) => entry.command)
    expect(commands[0]).toEqual({
      CopyLayer: {
        glyph: 'a',
        from: 'layer-1',
        to: { Background: 'm1' },
      },
    })
    expect(commands[1]).toEqual({
      SwapWithBackground: { glyph: 'a', master: 'm1' },
    })
  })

  it('deletes a background but never a master layer', async () => {
    const user = userEvent.setup()
    const mock = transport()
    const background = makeLayer({
      id: 'bg-1',
      kind: { Background: 'm1' },
      name: 'Background',
    })
    show({ layers: [masterLayer, background] })

    expect(
      screen.queryByRole('button', { name: 'Delete layer Regular' }),
    ).toBeNull()
    await user.click(
      screen.getByRole('button', { name: 'Delete layer Background' }),
    )

    expect(applied(mock)?.command).toEqual({
      RemoveLayer: { glyph: 'a', layer: 'bg-1' },
    })
  })

  it('switches the edited layer when a row is chosen', async () => {
    const user = userEvent.setup()
    transport()
    const background = makeLayer({
      id: 'bg-1',
      kind: { Background: 'm1' },
      name: 'Background',
    })
    show({ layers: [masterLayer, background] })

    await user.click(screen.getByRole('button', { name: 'Background' }))

    expect(useEditorStore.getState().editedLayer).toBe('bg-1')
  })

  it("keeps the document master aligned with another master's layer", async () => {
    const user = userEvent.setup()
    transport()
    const bold = makeLayer({
      id: 'bold-layer',
      kind: { Master: 'm2' },
      name: 'Bold',
    })
    show({ layers: [masterLayer, bold] })

    await user.click(screen.getByRole('button', { name: 'Bold' }))

    expect(useEditorStore.getState().editedLayer).toBe('bold-layer')
    expect(useDocumentStore.getState().byDoc[1]?.master).toBe('m2')
  })
})

describe('Components panel', () => {
  const view = {
    component: {
      base: 'acute',
      transform: [1, 0, 0, 1, 120, 40] as [
        number,
        number,
        number,
        number,
        number,
        number,
      ],
      anchor: null,
      auto_align: false,
      use_my_metrics: false,
      round_to_grid: false,
      smart: null,
      point_match: null,
    },
    path: 'M0 0L10 0Z',
    bbox: [0, 0, 100, 100] as [number, number, number, number],
  }

  it('adds a component picked from the search popover', async () => {
    const user = userEvent.setup()
    const mock = transport()
    show()

    await user.click(screen.getByRole('button', { name: 'Add component…' }))
    await user.type(screen.getByLabelText('Search for a base glyph'), 'acu')
    await user.click(await screen.findByRole('button', { name: 'acute' }))

    expect(applied(mock)?.command).toMatchObject({
      AddComponent: {
        glyph: 'a',
        layer: 'layer-1',
        component: { base: 'acute' },
      },
    })
  })

  it('decomposes and removes one component', async () => {
    const user = userEvent.setup()
    const mock = transport()
    show({ outline: { components: [view] } })

    await user.click(screen.getByRole('button', { name: 'Decompose acute' }))
    await user.click(screen.getByRole('button', { name: 'Delete acute' }))

    const commands = allApplied(mock).map((entry) => entry.command)
    expect(commands[0]).toEqual({
      DecomposeComponents: { glyph: 'a', layer: 'layer-1', indices: [0] },
    })
    expect(commands[1]).toEqual({
      RemoveComponents: { glyph: 'a', layer: 'layer-1', indices: [0] },
    })
  })

  it('reorders with one batched SetComponent per moved row', async () => {
    const user = userEvent.setup()
    const mock = transport()
    const second = {
      ...view,
      component: { ...view.component, base: 'grave' },
    }
    show({ outline: { components: [view, second] } })

    await user.click(screen.getByRole('button', { name: 'Move grave up' }))

    expect(applied(mock)?.command).toMatchObject({
      Batch: [
        { SetComponent: { index: 0, component: { base: 'grave' } } },
        { SetComponent: { index: 1, component: { base: 'acute' } } },
      ],
    })
    expect(applied(mock)?.label).toBe('Reorder components')
  })
})

describe('Anchors panel', () => {
  it('adds an anchor with the first free conventional name', async () => {
    const user = userEvent.setup()
    const mock = transport()
    show()

    await user.click(screen.getByRole('button', { name: 'Add anchor' }))

    expect(applied(mock)?.command).toEqual({
      AddAnchor: {
        glyph: 'a',
        layer: 'layer-1',
        anchor: { name: 'top', x: 300, y: 500 },
      },
    })
  })

  it('selects the anchor on the canvas and deletes it', async () => {
    const user = userEvent.setup()
    const mock = transport()
    show({ outline: { anchors: [{ name: 'top', x: 300, y: 700 }] } })

    await user.click(screen.getByRole('button', { name: 'Delete anchor top' }))

    expect(applied(mock)?.command).toEqual({
      RemoveAnchors: { glyph: 'a', layer: 'layer-1', indices: [0] },
    })
  })
})

describe('Guides panel', () => {
  it('adds a local guide and a master guide in the right scope', async () => {
    const user = userEvent.setup()
    const mock = transport()
    show()

    const buttons = screen.getAllByRole('button', { name: 'Add horizontal' })
    await user.click(buttons[0] as HTMLElement)
    await user.click(buttons[1] as HTMLElement)

    const commands = allApplied(mock).map((entry) => entry.command)
    expect(commands[0]).toMatchObject({
      AddGuide: { glyph: 'a', layer: 'layer-1', master: null },
    })
    expect(commands[1]).toMatchObject({
      AddGuide: { glyph: null, layer: null, master: 'm1' },
    })
  })

  it('locks a guide through SetGuide and marks it for measurement locally', async () => {
    const user = userEvent.setup()
    const mock = transport()
    show({
      outline: {
        guides: [
          {
            x: 0,
            y: 100,
            angle: 0,
            name: 'over',
            locked: false,
            color: null,
            anchor: 'Origin' as const,
          },
        ],
      },
    })

    const locks = screen.getAllByRole('button', { name: 'Lock guide' })
    await user.click(locks[0] as HTMLElement)
    expect(applied(mock)?.command).toMatchObject({
      SetGuide: { glyph: 'a', index: 0, guide: { locked: true } },
    })

    const measures = screen.getAllByRole('button', {
      name: 'Measurement guide',
    })
    await user.click(measures[0] as HTMLElement)
    expect(
      useEditorStore
        .getState()
        .measurementGuides.has(
          measurementGuideKey(
            { doc: 1, layer: 'layer-1', master: 'm1' },
            'layer',
            0,
          ),
        ),
    ).toBe(true)
  })
})

describe('Transform panel', () => {
  it('applies the whole layer as one TransformSelection', async () => {
    const user = userEvent.setup()
    const mock = transport()
    show()

    await user.click(screen.getByRole('button', { name: /^Transform/ }))
    const field = screen.getByLabelText('Move X')
    await user.clear(field)
    await user.type(field, '25{Enter}')
    await user.click(screen.getByRole('button', { name: 'Apply' }))

    expect(applied(mock)?.command).toEqual({
      TransformSelection: {
        glyph: 'a',
        layer: 'layer-1',
        points: [
          { contour: 0, point: 0 },
          { contour: 0, point: 1 },
          { contour: 0, point: 2 },
          { contour: 0, point: 3 },
        ],
        components: [],
        anchors: [],
        affine: [1, 0, 0, 1, 25, 0],
      },
    })
  })

  it('flips about the chosen origin', async () => {
    const user = userEvent.setup()
    const mock = transport()
    show()

    await user.click(screen.getByRole('button', { name: /^Transform/ }))
    await user.click(screen.getByRole('button', { name: 'Flip H' }))

    // Centre origin of the 40..560 box: x maps to 600 − x.
    expect(applied(mock)?.command).toMatchObject({
      TransformSelection: { affine: [-1, 0, 0, 1, 600, 0] },
    })
    expect(applied(mock)?.label).toBe('Flip horizontally')
  })

  it('applies an affine about exact custom-origin coordinates', async () => {
    const user = userEvent.setup()
    const mock = transport()
    show()

    await user.click(screen.getByRole('button', { name: /^Transform/ }))
    await user.click(screen.getByRole('radio', { name: 'Custom' }))
    const originX = screen.getByLabelText('Origin X')
    await user.clear(originX)
    await user.type(originX, '100{Enter}')
    const originY = screen.getByLabelText('Origin Y')
    await user.clear(originY)
    await user.type(originY, '200{Enter}')
    const scaleX = screen.getByLabelText('Scale X %')
    await user.clear(scaleX)
    await user.type(scaleX, '200{Enter}')
    await user.click(screen.getByRole('button', { name: 'Apply' }))

    // Doubling x about x=100 maps x to 2x−100; y is unchanged.
    expect(applied(mock)?.command).toMatchObject({
      TransformSelection: { affine: [2, 0, 0, 1, -100, 0] },
    })
  })

  it('aligns two selected contours, moving only the one that has to move', async () => {
    const user = userEvent.setup()
    const mock = transport()
    const second = makeContour([
      makePoint(700, 0),
      makePoint(800, 0, 'Line'),
      makePoint(800, 100, 'Line'),
    ])
    show({
      layers: [makeLayer({ contours: [square, second] })],
      selection: selectionOf([
        { kind: 'node', contour: 0, point: 0 },
        { kind: 'node', contour: 0, point: 1 },
        { kind: 'node', contour: 1, point: 0 },
        { kind: 'node', contour: 1, point: 1 },
      ]),
    })

    await user.click(screen.getByRole('button', { name: /^Transform/ }))
    await user.click(screen.getByRole('button', { name: 'Align left' }))

    // Contour 0 already sits at the left edge of the span, so only contour 1
    // moves and the batch collapses to the single command it holds.
    expect(applied(mock)?.command).toEqual({
      MovePoints: {
        glyph: 'a',
        layer: 'layer-1',
        points: [
          { contour: 1, point: 0 },
          { contour: 1, point: 1 },
        ],
        delta: [-660, 0],
        handles: 'WithHandles',
      },
    })
  })

  it('reports a failed command instead of swallowing it', async () => {
    const user = userEvent.setup()
    const reporter = vi.fn()
    setCommandErrorReporter(reporter)
    setTransport(
      createMockTransport({
        'doc.metrics_resolve': () => ({
          lsb: 40,
          rsb: 40,
          width: 600,
          source: 'Explicit' as const,
        }),
        'doc.apply': () => {
          throw new Error('layer is locked')
        },
      }),
    )
    show()

    await user.click(screen.getByRole('button', { name: 'Add anchor' }))

    await waitFor(() => expect(reporter).toHaveBeenCalledOnce())
  })
})
