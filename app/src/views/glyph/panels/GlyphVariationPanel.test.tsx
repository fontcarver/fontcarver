import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, expect, it } from 'vitest'
import { setTransport } from '../../../api'
import { useUiStore } from '../../../stores/uiStore'
import { makeMaster } from '../../../test/fontFixtures'
import { createMockTransport } from '../../../test/mockTransport'
import { GlyphVariationPanel } from './GlyphVariationPanel'

afterEach(() => setTransport(undefined))
it('scrubs the current glyph beside the editor without changing the document', async () => {
  useUiStore.setState({ sections: { variation: false } })
  const transport = createMockTransport({
    'variation.state': () => ({
      generation: 3,
      axes: [
        {
          tag: [119, 103, 104, 116],
          name: 'Weight',
          min: 100,
          default: 400,
          max: 900,
          map: [],
          hidden: false,
        },
      ],
      masters: [makeMaster()],
      instances: [],
      first_glyph: 'anotherGlyph',
    }),
    'variation.preview': (params) => ({
      generation: 3,
      path: 'M0 0L100 0L100 200Z',
      width: params.location.wght ?? 400,
      vertical_width: null,
      vertical_origin: null,
      units_per_em: 1000,
      sources: [],
      issues: [],
    }),
  })
  setTransport(transport)
  render(<GlyphVariationPanel doc={1} glyph="uni6C34" />)
  await userEvent.click(
    screen.getByRole('button', { name: 'Interpolation preview' }),
  )
  const slider = await screen.findByRole('slider', { name: 'Weight' })
  expect(screen.getByRole('textbox', { name: 'Glyph name' })).toHaveAttribute(
    'readonly',
  )
  fireEvent.change(slider, { target: { value: '700' } })
  await waitFor(() =>
    expect(transport.calls).toContainEqual(
      expect.objectContaining({
        method: 'variation.preview',
        params: expect.objectContaining({
          glyph: 'uni6C34',
          location: { wght: 700 },
        }),
      }),
    ),
  )
  expect(
    transport.calls.some((call) => call.method === 'variation.apply'),
  ).toBe(false)
})
