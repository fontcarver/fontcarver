import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { setTransport, type Features, type GlyphCell } from '../../api'
import { cellFor } from '../../test/glyphFixtures'
import { createMockTransport } from '../../test/mockTransport'

import { FeatureClassPreview } from './FeatureClassPreview'

const source: Features = {
  prefixes: [],
  features: [],
  source_of_truth: 'Fea',
  classes: [{ id: 'caps', name: 'Caps', code: '[A B]', automatic: false }],
}
const props = {
  doc: 1,
  master: 'm1',
  generation: 1,
  source,
  name: 'Caps',
  onClose: vi.fn(),
}

afterEach(() => {
  setTransport(undefined)
  vi.clearAllMocks()
})

describe('FeatureClassPreview', () => {
  it('shows exact member outlines in a labelled, keyboard closable region', async () => {
    const transport = createMockTransport({
      'glyphs.search': ({ query }) => [
        cellFor(query === 'A' ? 1 : 2, { name: query }),
      ],
      'glyphs.cell': ({ gid }) => cellFor(gid, { name: gid === 1 ? 'A' : 'B' }),
    })
    setTransport(transport)
    render(<FeatureClassPreview {...props} />)
    expect(screen.getByRole('region', { name: 'Class @Caps' })).toBeVisible()
    expect(screen.getByText('2 members')).toBeVisible()
    await waitFor(() => expect(screen.getAllByRole('img')).toHaveLength(2))
    expect(transport.calls).toContainEqual({
      method: 'glyphs.cell',
      params: { doc: 1, master: 'm1', gid: 1 },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Close class preview' }))
    expect(props.onClose).toHaveBeenCalledOnce()
    fireEvent.keyDown(screen.getByRole('region'), { key: 'Escape' })
    expect(props.onClose).toHaveBeenCalledTimes(2)
  })

  it('keeps missing and failed glyph names visible without substituting fuzzy search matches', async () => {
    const transport = createMockTransport({
      'glyphs.search': ({ query }) => {
        if (query === 'B') throw new Error('offline')
        return [cellFor(4, { name: 'A.alt' })]
      },
    })
    setTransport(transport)
    render(<FeatureClassPreview {...props} />)
    await screen.findByText('Some glyph names could not be resolved.')
    expect(screen.getByText('A')).toBeVisible()
    expect(screen.getByText('B')).toBeVisible()
    expect(screen.queryByRole('img')).not.toBeInTheDocument()
    expect(
      transport.calls.every((call) => call.method === 'glyphs.search'),
    ).toBe(true)
  })

  it('shows unsupported expressions and missing class references', () => {
    const input = {
      ...source,
      classes: [{ ...source.classes[0], code: '@Unknown' }],
    }
    render(<FeatureClassPreview {...props} source={input} />)
    expect(screen.getByText('Class @Unknown was not found.')).toBeVisible()
    expect(screen.getByText('This preview is incomplete.')).toBeVisible()
  })

  it('shows names without API requests when no preview master exists', () => {
    const transport = createMockTransport()
    setTransport(transport)
    render(<FeatureClassPreview {...props} master={null} />)
    expect(screen.getByText('A')).toBeVisible()
    expect(
      screen.getByText('Choose a preview master to see outlines.'),
    ).toBeVisible()
    expect(transport.calls).toEqual([])
  })

  it('limits visible previews to 64 names and searches to eight concurrent requests', async () => {
    let active = 0
    let peak = 0
    const finish: (() => void)[] = []
    const transport = createMockTransport({
      'glyphs.search': () =>
        new Promise<GlyphCell[]>((resolve) => {
          active += 1
          peak = Math.max(peak, active)
          finish.push(() => {
            active -= 1
            resolve([])
          })
        }),
    })
    setTransport(transport)
    const input = {
      ...source,
      classes: [{ ...source.classes[0], code: 'g000 - g099' }],
    }
    render(<FeatureClassPreview {...props} source={input} />)
    expect(screen.getByText('100 members')).toBeVisible()
    expect(screen.getByText('Showing the first 64 members.')).toBeVisible()
    expect(screen.getByText('g063')).toBeVisible()
    expect(screen.queryByText('g064')).not.toBeInTheDocument()
    expect(transport.calls).toHaveLength(8)
    for (let batch = 0; batch < 8; batch += 1) {
      await act(async () => finish.splice(0).forEach((resolve) => resolve()))
    }
    expect(transport.calls).toHaveLength(64)
    expect(peak).toBe(8)
  })

  it('discards old member lookups after document, master, generation and source changes', async () => {
    let finish: (cells: GlyphCell[]) => void = () => undefined
    const transport = createMockTransport({
      'glyphs.search': ({ doc }) =>
        doc === 1
          ? new Promise<GlyphCell[]>((resolve) => {
              finish = resolve
            })
          : [cellFor(3, { name: 'C' })],
      'glyphs.cell': ({ gid }) =>
        cellFor(gid, { name: 'C', path: 'M1 1L9 9Z' }),
    })
    setTransport(transport)
    const oldSource = {
      ...source,
      classes: [{ ...source.classes[0], code: 'A' }],
    }
    const newSource = {
      ...source,
      classes: [{ ...source.classes[0], code: 'C' }],
    }
    const view = render(<FeatureClassPreview {...props} source={oldSource} />)
    view.rerender(
      <FeatureClassPreview
        {...props}
        doc={2}
        master="m2"
        generation={2}
        source={newSource}
      />,
    )
    await screen.findByRole('img')
    await act(async () => finish([cellFor(1, { name: 'A' })]))
    expect(screen.queryByText('A')).not.toBeInTheDocument()
    expect(view.container.querySelector('path')).toHaveAttribute(
      'd',
      'M1 1L9 9Z',
    )
    expect(
      transport.calls.filter((call) => call.method === 'glyphs.cell'),
    ).toEqual([
      { method: 'glyphs.cell', params: { doc: 2, master: 'm2', gid: 3 } },
    ])
  })
})
