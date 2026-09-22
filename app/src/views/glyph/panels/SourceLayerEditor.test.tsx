import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, expect, it, vi } from 'vitest'
import { setTransport } from '../../../api'
import { makeLayer, makeMaster } from '../../../test/fontFixtures'
import { createMockTransport, noChange } from '../../../test/mockTransport'
import { SourceLayerEditor } from './SourceLayerEditor'

afterEach(() => setTransport(undefined))
it('initializes supplemental coordinates from the mapped default when the master omits them', async () => {
  const applied = vi.fn()
  const master = makeMaster({ id: 'm', location: {} })
  const transport = createMockTransport({
    'variation.state': () => ({
      generation: 7,
      masters: [master],
      instances: [],
      first_glyph: 'A',
      axes: [
        {
          tag: [119, 103, 104, 116],
          name: 'Weight',
          min: 100,
          default: 400,
          max: 900,
          hidden: false,
          map: [
            [100, 0],
            [900, 100],
          ],
        },
      ],
    }),
    'variation.apply': (params) => {
      expect(params.expected_generation).toBe(7)
      expect(params.command).toMatchObject({
        Layer: { kind: { Brace: { master: 'm', location: { wght: 50 } } } },
      })
      return noChange()
    },
  })
  setTransport(transport)
  render(
    <SourceLayerEditor
      doc={1}
      glyph="A"
      source={makeLayer({ kind: { Master: 'm' } })}
      master="m"
      duplicate
      smartAxes={[]}
      onClose={() => {}}
      onApplied={applied}
    />,
  )
  const field = await screen.findByLabelText('wght')
  expect(field).toHaveValue('37.5')
  const user = userEvent.setup()
  await user.clear(field)
  await user.type(field, '50')
  await user.click(screen.getByRole('button', { name: 'Apply changes' }))
  await waitFor(() => expect(applied).toHaveBeenCalledOnce())
})
