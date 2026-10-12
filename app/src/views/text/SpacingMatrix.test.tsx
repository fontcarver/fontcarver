import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it } from 'vitest'

import {
  setTransport,
  type Command,
  type MetricsKeys,
  type SpacingMatrix as SpacingMatrixResult,
  type SpacingMatrixParams,
  type SpacingRow,
} from '../../api'
import { initialUiState, useUiStore } from '../../stores/uiStore'
import { createMockTransport, noChange } from '../../test/mockTransport'
import { SpacingMatrix } from './SpacingMatrix'
import { useSpacingOverlay } from './useSpacingOverlay'
import { useSpacingViewStore } from './spacingView'
import { initialTextState } from './textState'
import type { PositionedGlyph } from './textLayout'

const NO_KEYS: MetricsKeys = {
  left: null,
  right: null,
  width: null,
  vertical_origin: null,
  vertical_width: null,
}

function glyph(name: string, gid: number, x: number): PositionedGlyph {
  return {
    compiledGid: gid,
    documentGid: gid as unknown as PositionedGlyph['documentGid'],
    name,
    editable: true,
    x,
    y: 0,
    xAdvance: 500,
    yAdvance: 0,
    sourceByte: x,
    sourceKind: 'literal',
  }
}

const rows: SpacingRow[] = [
  {
    glyph: 'n',
    layer: 'layer-n',
    width: 600,
    lsb: 40,
    rsb: 45,
    glyph_keys: NO_KEYS,
    layer_keys: null,
    resolved: { lsb: 40, rsb: 45, width: 600, source: 'Explicit' },
    key_error: null,
    out_of_sync: false,
    beam_left: 38,
    beam_right: 44,
    missing: false,
    locked: false,
  },
  {
    glyph: 'o',
    layer: 'layer-o',
    width: 610,
    lsb: 50,
    rsb: 50,
    glyph_keys: { ...NO_KEYS, left: '=n' },
    layer_keys: null,
    resolved: { lsb: 40, rsb: 50, width: 600, source: 'Key' },
    key_error: null,
    out_of_sync: true,
    beam_left: null,
    beam_right: null,
    missing: false,
    locked: false,
  },
]

let commands: Command[]
let requests: SpacingMatrixParams[]
/** The rows the mocked kernel answers with; a test may replace them. */
let served: SpacingRow[]

function Harness({
  glyphs,
  master = 'm1',
}: {
  glyphs: readonly PositionedGlyph[]
  master?: string | null
}) {
  const overlay = useSpacingOverlay({
    tab: 'tab-1',
    doc: 1,
    master,
    generation: 3,
    unitsPerEm: 1000,
    glyphs,
  })
  return (
    <SpacingMatrix
      doc={1}
      master={master}
      unitsPerEm={1000}
      overlay={overlay}
    />
  )
}

beforeEach(() => {
  commands = []
  requests = []
  served = rows
  useUiStore.setState(initialUiState())
  useSpacingViewStore.setState({ byTab: {} })
  setTransport(
    createMockTransport({
      'metrics.spacing': (params: SpacingMatrixParams): SpacingMatrixResult => {
        requests.push(params)
        return { generation: 3, master: 'm1', rows: served }
      },
      'doc.apply': ({ command }) => {
        commands.push(command)
        return noChange([1])
      },
    }),
  )
})

function open() {
  fireEvent.click(screen.getByText('Spacing matrix'))
}

describe('spacing matrix', () => {
  it('asks for nothing until the panel is opened', async () => {
    render(<Harness glyphs={[glyph('n', 1, 0), glyph('o', 2, 600)]} />)
    expect(requests).toEqual([])
    open()
    await screen.findByRole('table')
    expect(requests).toHaveLength(1)
    expect(requests[0]?.glyphs).toEqual(['n', 'o'])
    expect(requests[0]?.beam).toBeNull()
  })

  it('writes a typed number as absolute metrics for that layer', async () => {
    render(<Harness glyphs={[glyph('n', 1, 0)]} />)
    open()
    const cell = await screen.findByLabelText('LSB of n')
    fireEvent.change(cell, { target: { value: '55' } })
    fireEvent.keyDown(cell, { key: 'Enter' })
    await waitFor(() => expect(commands).toHaveLength(1))
    expect(commands[0]).toEqual({
      SetMetrics: {
        glyph: 'n',
        layer: 'layer-n',
        width: null,
        lsb: 55,
        rsb: null,
      },
    })
  })

  it('writes a typed formula as a glyph-wide metrics key', async () => {
    render(<Harness glyphs={[glyph('n', 1, 0)]} />)
    open()
    const cell = await screen.findByLabelText('RSB of n')
    fireEvent.change(cell, { target: { value: 'H' } })
    fireEvent.keyDown(cell, { key: 'Enter' })
    await waitFor(() => expect(commands).toHaveLength(1))
    expect(commands[0]).toEqual({
      SetMetricsKeys: { glyph: 'n', keys: { ...NO_KEYS, right: '=H' } },
    })
  })

  it('replaces a formula when a number is typed over it, as one entry', async () => {
    render(<Harness glyphs={[glyph('o', 2, 0)]} />)
    open()
    const cell = await screen.findByLabelText('LSB of o')
    expect(cell).toHaveValue('=n')
    fireEvent.change(cell, { target: { value: '12' } })
    fireEvent.keyDown(cell, { key: 'Enter' })
    await waitFor(() => expect(commands).toHaveLength(1))
    expect(commands[0]).toEqual({
      Batch: [
        { SetMetricsKeys: { glyph: 'o', keys: NO_KEYS } },
        {
          SetMetrics: {
            glyph: 'o',
            layer: 'layer-o',
            width: null,
            lsb: 12,
            rsb: null,
          },
        },
      ],
    })
  })

  it('refuses a malformed formula in place and applies nothing', async () => {
    render(<Harness glyphs={[glyph('n', 1, 0)]} />)
    open()
    const cell = await screen.findByLabelText('LSB of n')
    fireEvent.change(cell, { target: { value: '=H+' } })
    fireEvent.keyDown(cell, { key: 'Enter' })
    expect(cell).toHaveAttribute('aria-invalid', 'true')
    expect(commands).toEqual([])
  })

  it('offers Update only on a row whose formulas disagree with its metrics', async () => {
    render(<Harness glyphs={[glyph('n', 1, 0), glyph('o', 2, 600)]} />)
    open()
    await screen.findByRole('table')
    expect(screen.getAllByText('In sync')).toHaveLength(1)
    fireEvent.click(screen.getByRole('button', { name: 'Update' }))
    await waitFor(() => expect(commands).toHaveLength(1))
    expect(commands[0]).toEqual({
      UpdateMetrics: { glyph: 'o', master: 'm1' },
    })
  })

  it('measures at the beam height and shows each row s margins', async () => {
    render(<Harness glyphs={[glyph('n', 1, 0)]} />)
    open()
    await screen.findByRole('table')
    fireEvent.click(screen.getByLabelText('Beam', { selector: 'input' }))
    await waitFor(() => expect(requests).toHaveLength(2))
    expect(requests[1]?.beam).toBe(250)
    expect(await screen.findByText('38')).toBeInTheDocument()
    fireEvent.change(screen.getByLabelText('Drag the beam height'), {
      target: { value: '400' },
    })
    await waitFor(() => expect(requests).toHaveLength(3))
    expect(requests[2]?.beam).toBe(400)
  })

  it('clears both formula scopes when a number replaces a local formula', async () => {
    served = [
      {
        ...rows[0]!,
        glyph_keys: { ...NO_KEYS, left: '=H' },
        layer_keys: { ...NO_KEYS, left: '==120' },
      },
    ]
    render(<Harness glyphs={[glyph('n', 1, 0)]} />)
    open()
    const cell = await screen.findByLabelText('LSB of n')
    // The cell shows the effective formula, which is the layer-local one.
    expect(cell).toHaveValue('==120')
    fireEvent.change(cell, { target: { value: '42' } })
    fireEvent.keyDown(cell, { key: 'Enter' })
    await waitFor(() => expect(commands).toHaveLength(1))
    // Clearing only the local key would hand the cell back to `=H`, leaving the
    // row out of sync with the number just typed.
    expect(commands[0]).toEqual({
      Batch: [
        {
          Batch: [
            {
              SetLayerMetricsKeys: { glyph: 'n', layer: 'layer-n', keys: null },
            },
            { SetMetricsKeys: { glyph: 'n', keys: NO_KEYS } },
          ],
        },
        {
          SetMetrics: {
            glyph: 'n',
            layer: 'layer-n',
            width: null,
            lsb: 42,
            rsb: null,
          },
        },
      ],
    })
  })

  it('clears both formula scopes when the cell is emptied', async () => {
    served = [
      {
        ...rows[0]!,
        glyph_keys: { ...NO_KEYS, left: '=H' },
        layer_keys: { ...NO_KEYS, left: '==120' },
      },
    ]
    render(<Harness glyphs={[glyph('n', 1, 0)]} />)
    open()
    const cell = await screen.findByLabelText('LSB of n')
    fireEvent.change(cell, { target: { value: '' } })
    fireEvent.keyDown(cell, { key: 'Enter' })
    await waitFor(() => expect(commands).toHaveLength(1))
    expect(commands[0]).toEqual({
      Batch: [
        { SetLayerMetricsKeys: { glyph: 'n', layer: 'layer-n', keys: null } },
        { SetMetricsKeys: { glyph: 'n', keys: NO_KEYS } },
      ],
    })
  })

  it('never updates one master before a master is selected', async () => {
    render(<Harness glyphs={[glyph('o', 2, 0)]} master={null} />)
    open()
    const button = await screen.findByRole('button', { name: 'Update' })
    expect(button).toBeDisabled()
    fireEvent.click(button)
    await new Promise((resolve) => setTimeout(resolve, 10))
    expect(commands).toEqual([])
  })

  it('says why a malformed cell was not applied', async () => {
    render(<Harness glyphs={[glyph('n', 1, 0)]} />)
    open()
    const cell = await screen.findByLabelText('LSB of n')
    fireEvent.change(cell, { target: { value: '=H@x' } })
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'takes a number after @',
    )
    fireEvent.keyDown(cell, { key: 'Enter' })
    expect(commands).toEqual([])
  })

  it('locks every cell while the text tab locks spacing', async () => {
    useUiStore.getState().openTab({
      kind: 'glyph',
      doc: 1,
      gid: 1,
      textState: { ...initialTextState('m1', 'no'), kerningMode: 'locked' },
    })
    render(<Harness glyphs={[glyph('n', 1, 0)]} />)
    open()
    const cell = await screen.findByLabelText('LSB of n')
    expect(cell).toBeDisabled()
    fireEvent.change(cell, { target: { value: '99' } })
    fireEvent.keyDown(cell, { key: 'Enter' })
    expect(commands).toEqual([])
  })
})
