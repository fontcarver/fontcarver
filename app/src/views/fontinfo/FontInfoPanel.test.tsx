import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { setTransport } from '../../api'
import { forgetFontInfo } from '../../stores/fontInfoCache'
import {
  makeFontInfo,
  makeFontInfoView,
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

    await user.click(screen.getByRole('tab', { name: 'Axes' }))
    expect(
      screen.getByText('This font has no variation axes.'),
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
