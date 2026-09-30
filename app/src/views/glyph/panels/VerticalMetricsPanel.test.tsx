import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { setTransport, type ApplyParams, type Layer } from '../../../api'
import { setCommandErrorReporter } from '../../../shell/commands'
import {
  makeGlyph,
  makeGlyphDetail,
  makeLayer,
  makeLayerOutline,
  makeMaster,
  makeMasterMetrics,
} from '../../../test/fontFixtures'
import {
  createMockTransport,
  noChange,
  type MockTransport,
} from '../../../test/mockTransport'
import { buildScene } from '../glyphScene'

import { VerticalMetricsPanel } from './VerticalMetricsPanel'

function show(
  options: {
    layer?: Partial<Layer>
    bbox?: [number, number, number, number] | null
    reject?: boolean
  } = {},
) {
  const layer = makeLayer(options.layer)
  const glyph = makeGlyph({ layers: [layer] })
  const scene = buildScene({
    detail: makeGlyphDetail(glyph),
    cell: null,
    master: makeMaster({
      metrics: makeMasterMetrics({
        default_vertical_advance: 1100,
        default_vertical_origin: 880,
      }),
    }),
    masterId: 'm1',
    unitsPerEm: 1000,
  })
  const reload = vi.fn()
  const errors = vi.fn()
  const mock = createMockTransport({
    'doc.apply': () => {
      if (options.reject) throw new Error('Vertical advance is out of range')
      return noChange([42])
    },
  })
  setTransport(mock)
  setCommandErrorReporter(errors)
  render(
    <VerticalMetricsPanel
      target={{ doc: 1, glyph: glyph.name, layer: layer.id }}
      glyph={glyph}
      scene={scene}
      outline={makeLayerOutline(layer, {
        bbox: options.bbox === undefined ? [40, -100, 560, 700] : options.bbox,
      })}
      reload={reload}
    />,
  )
  return { mock, reload, errors, layer, glyph }
}

function edits(mock: MockTransport): ApplyParams[] {
  return mock.calls
    .filter((call) => call.method === 'doc.apply')
    .map((call) => call.params as ApplyParams)
}

async function enter(label: string, value: string) {
  const user = userEvent.setup()
  const input = screen.getByRole('textbox', { name: label })
  await user.clear(input)
  await user.type(input, `${value}{Enter}`)
}

afterEach(() => {
  setTransport(undefined)
  setCommandErrorReporter(() => undefined)
})

describe('vertical metrics inspector', () => {
  it('derives both sidebearings from effective master metrics and resolved ink bounds', () => {
    show()
    expect(screen.getByRole('textbox', { name: 'TSB' })).toHaveValue('180')
    expect(screen.getByRole('textbox', { name: 'BSB' })).toHaveValue('120')
    expect(screen.getByRole('textbox', { name: 'vert. origin' })).toHaveValue(
      '880',
    )
    expect(screen.getByRole('textbox', { name: 'vert. advance' })).toHaveValue(
      '1100',
    )
    expect(
      screen.getByRole('button', { name: 'Use master vertical metrics' }),
    ).toBeDisabled()
  })

  it('changes origin for TSB while retaining inherited advance', async () => {
    const { mock, glyph, layer } = show()
    await enter('TSB', '210')
    await waitFor(() => expect(edits(mock)).toHaveLength(1))
    expect(edits(mock)[0]?.command).toEqual({
      SetVerticalMetrics: {
        glyph: glyph.name,
        layer: layer.id,
        advance: null,
        origin: 910,
      },
    })
  })

  it('changes advance for BSB while retaining explicit zero origin', async () => {
    const { mock, glyph, layer } = show({ layer: { vertical_origin: 0 } })
    await enter('BSB', '250')
    await waitFor(() => expect(edits(mock)).toHaveLength(1))
    expect(edits(mock)[0]?.command).toEqual({
      SetVerticalMetrics: {
        glyph: glyph.name,
        layer: layer.id,
        advance: 350,
        origin: 0,
      },
    })
  })

  it('retains the other override when changing advance or origin directly', async () => {
    const { mock } = show({
      layer: { vertical_width: 1300, vertical_origin: 900 },
    })
    await enter('vert. advance', '1200')
    await waitFor(() => expect(edits(mock)).toHaveLength(1))
    expect(edits(mock)[0]?.command).toMatchObject({
      SetVerticalMetrics: { advance: 1200, origin: 900 },
    })
    await enter('vert. origin', '920')
    await waitFor(() => expect(edits(mock)).toHaveLength(2))
    expect(edits(mock)[1]?.command).toMatchObject({
      SetVerticalMetrics: { advance: 1300, origin: 920 },
    })
  })

  it('clears both overrides in one command', async () => {
    const { mock } = show({ layer: { vertical_width: 0, vertical_origin: 0 } })
    await userEvent
      .setup()
      .click(
        screen.getByRole('button', { name: 'Use master vertical metrics' }),
      )
    await waitFor(() => expect(edits(mock)).toHaveLength(1))
    expect(edits(mock)[0]?.command).toMatchObject({
      SetVerticalMetrics: { advance: null, origin: null },
    })
  })

  it('disables sidebearing edits for empty ink while retaining advance and origin editing', () => {
    show({ bbox: null })
    expect(screen.getByRole('textbox', { name: 'TSB' })).toBeDisabled()
    expect(screen.getByRole('textbox', { name: 'BSB' })).toBeDisabled()
    expect(screen.getByRole('textbox', { name: 'vert. advance' })).toBeEnabled()
    expect(screen.getByRole('textbox', { name: 'vert. origin' })).toBeEnabled()
    expect(screen.getByText('Sidebearings require an outline.')).toBeVisible()
  })

  it('rejects invalid numeric input without applying a command', async () => {
    const { mock } = show()
    await enter('TSB', 'oops')
    expect(edits(mock)).toHaveLength(0)
    expect(screen.getByRole('textbox', { name: 'TSB' })).toHaveValue('180')
  })

  it('reconciles a rejected value even when the model remains unchanged', async () => {
    const { mock, reload, errors } = show({ reject: true })
    await enter('BSB', '70000')
    await waitFor(() => expect(reload).toHaveBeenCalledOnce())
    expect(errors).toHaveBeenCalledOnce()
    expect(screen.getByRole('textbox', { name: 'BSB' })).toHaveValue('120')
    expect(edits(mock)).toHaveLength(1)
  })
})
