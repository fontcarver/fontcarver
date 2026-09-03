import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { setTransport } from '../../api'
import { forgetFontInfo } from '../../stores/fontInfoCache'
import {
  makeFontInfo,
  makeFontInfoView,
  makeInstance,
  makeMaster,
  makeMasterMetrics,
} from '../../test/fontFixtures'
import { createMockTransport, makeSummary } from '../../test/mockTransport'

import { FontInfoPanel } from './FontInfoPanel'

const NAMES = [
  {
    name_id: 1,
    language: 'en',
    text: 'DejaVu Sans',
    platform: 'Windows' as const,
  },
  {
    name_id: 1,
    language: 'de',
    text: 'DejaVu Sans Deutsch',
    platform: 'Windows' as const,
  },
  {
    name_id: 6,
    language: 'en',
    text: 'DejaVuSans',
    platform: 'Mac' as const,
  },
]

function transportFor(
  view = makeFontInfoView({ info: makeFontInfo({ names: NAMES }) }),
) {
  return createMockTransport({ 'doc.font_info': () => view })
}

beforeEach(() => {
  forgetFontInfo(1)
})

afterEach(() => {
  setTransport(undefined)
  forgetFontInfo(1)
})

describe('FontInfoPanel', () => {
  it('shows the Font section from doc.font_info', async () => {
    setTransport(transportFor())
    render(<FontInfoPanel summary={makeSummary()} />)

    expect(await screen.findByText('DejaVu Sans')).toBeInTheDocument()
    expect(
      screen.getByText('Copyright (c) 2003 Bitstream Inc.'),
    ).toBeInTheDocument()
    expect(screen.getByText('2.037')).toBeInTheDocument()
  })

  it('reloads when a document event reports changed font info', async () => {
    let view = makeFontInfoView({
      info: makeFontInfo({ family_name: 'Before Edit' }),
    })
    const transport = createMockTransport({ 'doc.font_info': () => view })
    setTransport(transport)
    render(<FontInfoPanel summary={makeSummary()} />)
    expect(await screen.findByText('Before Edit')).toBeInTheDocument()

    view = makeFontInfoView({
      info: makeFontInfo({ family_name: 'After Edit' }),
    })
    transport.emit({
      doc: 1,
      generation: 2,
      change: {
        glyphs: [],
        font_info: true,
        masters: false,
        glyph_order: false,
        metrics: false,
        layers: false,
        guides: false,
      },
      undo_label: 'Edit font info',
      redo_label: null,
      dirty: true,
    })

    expect(await screen.findByText('After Edit')).toBeInTheDocument()
    expect(
      transport.calls.filter((call) => call.method === 'doc.font_info'),
    ).toHaveLength(2)
  })

  it('renders every name record, including non-default languages', async () => {
    const user = userEvent.setup()
    setTransport(transportFor())
    render(<FontInfoPanel summary={makeSummary()} />)
    await screen.findByText('DejaVu Sans')

    await user.click(screen.getByRole('tab', { name: 'Names' }))

    const table = screen.getByRole('table')
    expect(within(table).getAllByRole('row')).toHaveLength(NAMES.length + 1)
    expect(within(table).getByText('DejaVu Sans Deutsch')).toBeInTheDocument()
    expect(within(table).getByText('de')).toBeInTheDocument()
    expect(
      within(table).getAllByText('PostScript name').length,
    ).toBeGreaterThan(0)
  })

  it('shows one metrics column per master', async () => {
    const user = userEvent.setup()
    setTransport(
      transportFor(
        makeFontInfoView({
          info: makeFontInfo({ names: NAMES }),
          masters: [
            makeMaster({ id: 'm1', name: 'Light' }),
            makeMaster({
              id: 'm2',
              name: 'Bold',
              metrics: makeMasterMetrics({ x_height: 520 }),
            }),
          ],
        }),
      ),
    )
    render(<FontInfoPanel summary={makeSummary()} />)
    await screen.findByText('DejaVu Sans')

    await user.click(screen.getByRole('tab', { name: 'Masters' }))

    const table = screen.getByRole('table')
    expect(
      within(table).getByRole('columnheader', { name: 'Light' }),
    ).toBeInTheDocument()
    expect(
      within(table).getByRole('columnheader', { name: 'Bold' }),
    ).toBeInTheDocument()
    const row = within(table).getByRole('row', { name: /x-height/ })
    expect(within(row).getByText('520')).toBeInTheDocument()
  })

  it('searches across every section at once', async () => {
    const user = userEvent.setup()
    setTransport(transportFor())
    render(<FontInfoPanel summary={makeSummary()} />)
    await screen.findByText('DejaVu Sans')

    await user.type(
      screen.getByRole('searchbox', { name: 'Search fields' }),
      'weight',
    )

    // The Other section is reached without leaving the Font section first.
    expect(screen.getByText('Weight class')).toBeInTheDocument()
    expect(screen.getByText('400')).toBeInTheDocument()
    expect(screen.queryByText('Copyright (c) 2003 Bitstream Inc.')).toBeNull()
    // Only the section that actually matched is titled and shown.
    expect(screen.getByRole('heading', { name: 'Other' })).toBeInTheDocument()
    expect(screen.queryByRole('heading', { name: 'Names' })).toBeNull()
    expect(screen.queryByText('This font has no name records.')).toBeNull()
  })

  it('reports a font with no variation axes', async () => {
    const user = userEvent.setup()
    setTransport(transportFor())
    render(<FontInfoPanel summary={makeSummary()} />)
    await screen.findByText('DejaVu Sans')

    await user.click(screen.getByRole('tab', { name: 'Axes / Instances' }))
    expect(
      screen.getByText('This font has no variation axes.'),
    ).toBeInTheDocument()
    expect(screen.getByText('This font has no instances.')).toBeInTheDocument()
  })

  it('shows every named instance and its per-axis location', async () => {
    const user = userEvent.setup()
    setTransport(
      transportFor(
        makeFontInfoView({
          instances: [
            makeInstance({
              name: 'Text Regular',
              style_name: 'Regular',
              location: { wght: 400, wdth: 100 },
              active: true,
            }),
            makeInstance({
              name: 'Display Semibold',
              style_name: 'Semibold',
              family_name: 'DejaVu Display',
              location: { wght: 650, wdth: 90 },
              active: false,
              is_variable_named_instance: true,
            }),
          ],
        }),
      ),
    )
    render(
      <FontInfoPanel
        summary={makeSummary({
          axes: [
            {
              tag: 'wght',
              name: 'Weight',
              min: 100,
              default: 400,
              max: 900,
              hidden: false,
            },
            {
              tag: 'wdth',
              name: 'Width',
              min: 75,
              default: 100,
              max: 125,
              hidden: false,
            },
          ],
        })}
      />,
    )
    await screen.findByText('DejaVu Sans')

    await user.click(screen.getByRole('tab', { name: 'Axes / Instances' }))

    const table = screen.getByRole('table', { name: 'Instances' })
    expect(within(table).getAllByRole('row')).toHaveLength(3)
    expect(within(table).getByText('Text Regular')).toBeInTheDocument()
    const display = within(table).getByRole('row', {
      name: /Display Semibold/,
    })
    expect(within(display).getByText('Semibold')).toBeInTheDocument()
    expect(within(display).getByText('DejaVu Display')).toBeInTheDocument()
    expect(within(display).getByText('650')).toBeInTheDocument()
    expect(within(display).getByText('90')).toBeInTheDocument()
    expect(within(display).getByText('No')).toBeInTheDocument()
    expect(within(display).getByText('Yes')).toBeInTheDocument()
  })

  it('shows the table inventory sorted by tag and marks preserved tables', async () => {
    const user = userEvent.setup()
    setTransport(
      transportFor(
        makeFontInfoView({
          tables: [
            { tag: 'name', len: 15_623, preserved: false },
            { tag: 'GSUB', len: 5_598, preserved: true },
            { tag: 'cmap', len: 7_056, preserved: false },
          ],
        }),
      ),
    )
    render(<FontInfoPanel summary={makeSummary()} />)
    await screen.findByText('DejaVu Sans')

    await user.click(screen.getByRole('tab', { name: 'Tables' }))

    const table = screen.getByRole('table', { name: 'OpenType tables' })
    const rows = within(table).getAllByRole('row').slice(1)
    expect(
      rows.map((row) => within(row).getAllByRole('cell')[0]?.textContent),
    ).toEqual(['cmap', 'GSUB', 'name'])
    expect(within(rows[0]!).getByText('7,056 bytes')).toBeInTheDocument()
    expect(within(rows[1]!).getByText('Preserved')).toBeInTheDocument()
    expect(within(rows[2]!).queryByText('Preserved')).toBeNull()
  })

  it('shows an empty state when no imported table inventory is available', async () => {
    const user = userEvent.setup()
    setTransport(transportFor(makeFontInfoView({ tables: [] })))
    render(<FontInfoPanel summary={makeSummary()} />)
    await screen.findByText('DejaVu Sans')

    await user.click(screen.getByRole('tab', { name: 'Tables' }))

    expect(
      screen.getByText('This document has no imported table inventory.'),
    ).toBeInTheDocument()
  })

  it('surfaces a transport failure instead of an empty panel', async () => {
    setTransport(
      createMockTransport({
        'doc.font_info': () => {
          throw new Error('document 1 is not open')
        },
      }),
    )
    render(<FontInfoPanel summary={makeSummary()} />)
    expect(
      await screen.findByText('document 1 is not open'),
    ).toBeInTheDocument()
  })
})
