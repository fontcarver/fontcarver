import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { setTransport, type LayerEffectsResult } from '../../../api'
import { makeLayer, makeCell } from '../../../test/fontFixtures'
import { createMockTransport } from '../../../test/mockTransport'
import { ReusableConstructionEditor } from './ReusableConstructionEditor'
import '../../../i18n'

const example = {
  glyph: 'H',
  master: 'Regular',
  before: 'M0 0L100 0L100 100Z',
  after: 'M0 0L110 0L110 110Z',
  before_width: 600,
  after_width: 600,
  bounds: [0, 0, 110, 110] as [number, number, number, number],
}
const result: LayerEffectsResult = {
  generation: 7,
  example,
  approximated_layers: [],
  change: null,
}

afterEach(() => setTransport(undefined))

describe('reusable construction editor', () => {
  it('requires a preview and writes the captured generation', async () => {
    const user = userEvent.setup()
    const onApplied = vi.fn()
    const onClose = vi.fn()
    const mock = createMockTransport({ 'constructions.apply': () => result })
    setTransport(mock)
    render(
      <ReusableConstructionEditor
        target={{ doc: 1, glyph: 'H', layer: 'layer-1' }}
        layer={makeLayer()}
        points={[]}
        onClose={onClose}
        onApplied={onApplied}
      />,
    )
    expect(screen.getByRole('button', { name: 'Keep live' })).toBeDisabled()
    await user.click(screen.getByRole('button', { name: 'Preview' }))
    await user.click(await screen.findByRole('button', { name: 'Keep live' }))
    await waitFor(() => expect(onApplied).toHaveBeenCalledOnce())
    expect(
      mock.calls
        .filter((call) => call.method === 'constructions.apply')
        .map((call) => call.params),
    ).toEqual([
      {
        doc: 1,
        glyph: 'H',
        layer: 'layer-1',
        constructions: [],
        add: null,
        dry_run: true,
        bake: false,
        expected_generation: null,
      },
      {
        doc: 1,
        glyph: 'H',
        layer: 'layer-1',
        constructions: [],
        add: null,
        dry_run: false,
        bake: false,
        expected_generation: 7,
      },
    ])
    expect(onClose).toHaveBeenCalledOnce()
  })

  it('rejects a source attachment without the required selected carrier nodes', async () => {
    const user = userEvent.setup()
    const mock = createMockTransport({
      'constructions.apply': () => result,
      'glyphs.search': () => [makeCell({ name: '_corner.test' })],
    })
    setTransport(mock)
    render(
      <ReusableConstructionEditor
        target={{ doc: 1, glyph: 'H', layer: 'layer-1' }}
        layer={makeLayer()}
        points={[]}
        onClose={() => undefined}
        onApplied={() => undefined}
      />,
    )
    await user.click(
      screen.getByRole('button', { name: 'Choose source glyph' }),
    )
    await user.type(
      screen.getByRole('textbox', { name: 'Search for a base glyph' }),
      '_corner',
    )
    await user.click(
      await screen.findByRole('button', { name: '_corner.test' }),
    )
    await user.click(screen.getByRole('button', { name: 'Preview' }))
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Select one node',
    )
    expect(
      mock.calls.some((call) => call.method === 'constructions.apply'),
    ).toBe(false)
  })

  it('drops reviewed preview after changing construction parameters', async () => {
    const user = userEvent.setup()
    setTransport(createMockTransport({ 'constructions.apply': () => result }))
    render(
      <ReusableConstructionEditor
        target={{ doc: 1, glyph: 'H', layer: 'layer-1' }}
        layer={makeLayer()}
        points={[]}
        onClose={() => undefined}
        onApplied={() => undefined}
      />,
    )
    await user.click(screen.getByRole('button', { name: 'Preview' }))
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Keep live' })).toBeEnabled(),
    )
    await user.selectOptions(
      screen.getByRole('combobox', { name: 'Attachment kind' }),
      'brush',
    )
    expect(screen.getByRole('button', { name: 'Keep live' })).toBeDisabled()
  })
})
