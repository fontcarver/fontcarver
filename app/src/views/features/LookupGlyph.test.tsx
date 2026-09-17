import { act, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'

import { setTransport, type GlyphCell } from '../../api'
import { cellFor } from '../../test/glyphFixtures'
import { createMockTransport } from '../../test/mockTransport'

import { LookupGlyph } from './LookupGlyph'

afterEach(() => setTransport(undefined))

describe('LookupGlyph', () => {
  it('renders the actual selected-master outline beside its persistent name', async () => {
    const transport = createMockTransport({
      'glyphs.cell': () => cellFor(3, { name: 'a.alt' }),
    })
    setTransport(transport)
    const view = render(
      <LookupGlyph
        doc={1}
        master="m2"
        generation={3}
        glyph={{ gid: 3, name: 'a.alt' }}
      />,
    )
    expect(screen.getByText('a.alt')).toBeVisible()
    await screen.findByRole('img')
    expect(view.container.querySelector('path')).toHaveAttribute(
      'd',
      'M0 0L100 0L100 100Z',
    )
    expect(transport.calls).toEqual([
      { method: 'glyphs.cell', params: { doc: 1, master: 'm2', gid: 3 } },
    ])
  })

  it('drops old outlines after document, master or generation changes', async () => {
    let finish: (cell: GlyphCell) => void = () => undefined
    setTransport(
      createMockTransport({
        'glyphs.cell': ({ doc }) =>
          doc === 1
            ? new Promise<GlyphCell>((resolve) => {
                finish = resolve
              })
            : cellFor(3, { name: 'a.alt', path: 'M1 1L9 9Z' }),
      }),
    )
    const view = render(
      <LookupGlyph
        doc={1}
        master="m1"
        generation={1}
        glyph={{ gid: 3, name: 'a.alt' }}
      />,
    )
    view.rerender(
      <LookupGlyph
        doc={2}
        master="m2"
        generation={2}
        glyph={{ gid: 3, name: 'a.alt' }}
      />,
    )
    await screen.findByRole('img')
    await act(async () => finish(cellFor(3, { name: 'a.alt' })))
    expect(view.container.querySelector('path')).toHaveAttribute(
      'd',
      'M1 1L9 9Z',
    )
  })

  it('keeps unresolved names visible without guessing a glyph', () => {
    const transport = createMockTransport()
    setTransport(transport)
    render(
      <LookupGlyph
        doc={1}
        master="m1"
        generation={1}
        glyph={{ gid: null, name: 'missing' }}
      />,
    )
    expect(screen.getByText('missing')).toBeVisible()
    expect(screen.queryByRole('img')).not.toBeInTheDocument()
    expect(transport.calls).toEqual([])
  })
})
