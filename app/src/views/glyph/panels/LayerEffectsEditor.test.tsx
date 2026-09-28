import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, expect, it, vi } from 'vitest'
import { setTransport, type Glyph, type Layer } from '../../../api'
import { createMockTransport, noChange } from '../../../test/mockTransport'
import { LayerEffectsEditor } from './LayerEffectsEditor'

const layer: Layer = {
  id: 'l',
  kind: { Master: 'm' },
  name: 'Regular',
  width: 600,
  vertical_width: null,
  vertical_origin: null,
  contours: [],
  components: [],
  anchors: [],
  guides: [],
  background_image: null,
  hints: { h_stems: [], v_stems: [], d_stems: [], flex: false, hint_masks: [] },
  tt_instructions: null,
  color_layers: null,
  user_data: null,
}
const glyph = { name: 'A', layers: [layer] } as Glyph
const result = {
  generation: 19,
  example: {
    glyph: 'A',
    master: 'Regular',
    before: 'M0 0L100 100',
    after: 'M0 0L100 0L100 100Z',
    before_width: 600,
    after_width: 600,
    bounds: [0, 0, 100, 100] as [number, number, number, number],
  },
  approximated_layers: [],
  change: null,
}
afterEach(() => setTransport(undefined))

it('invalidates reviewed drafts after reordering and bakes the captured layer revision', async () => {
  const close = vi.fn(),
    applied = vi.fn()
  const mock = createMockTransport({
    'effects.apply': (params) => {
      expect(params.layer).toBe('l')
      expect(params.glyph).toBe('A')
      expect(params.doc).toBe(7)
      return { ...result, change: params.dry_run ? null : noChange() }
    },
  })
  setTransport(mock)
  render(
    <LayerEffectsEditor
      doc={7}
      glyph={glyph}
      layer={layer}
      upem={1000}
      onClose={close}
      onApplied={applied}
    />,
  )
  const user = userEvent.setup()
  const bake = screen.getByRole('button', { name: 'Bake result' })
  expect(bake).toBeDisabled()
  await user.click(screen.getByRole('button', { name: 'Add stroke' }))
  await user.click(screen.getByRole('button', { name: 'Add overlap removal' }))
  await user.click(screen.getByRole('button', { name: 'Preview changes' }))
  await screen.findByText(
    'Preview ready. Apply or bake this reviewed revision.',
  )
  expect(bake).toBeEnabled()
  await user.click(screen.getByRole('button', { name: 'Move effect 2 up' }))
  expect(bake).toBeDisabled()
  await user.click(screen.getByRole('button', { name: 'Preview changes' }))
  await screen.findByText(
    'Preview ready. Apply or bake this reviewed revision.',
  )
  await user.click(bake)
  const last = mock.calls.filter((c) => c.method === 'effects.apply').at(-1)!
  expect(last.params).toMatchObject({
    bake: true,
    dry_run: false,
    expected_generation: 19,
    effects: [
      { effect: { kind: 'remove_overlap' } },
      { effect: { kind: 'stroke' } },
    ],
  })
  expect(close).toHaveBeenCalledOnce()
  expect(applied).toHaveBeenCalledOnce()
})

it('captures a stored mask and requires a new review after stale apply', async () => {
  const close = vi.fn()
  const mock = createMockTransport({
    'effects.mask': () => 'M0 0L40 0L40 40Z',
    'effects.apply': (params) => {
      if (!params.dry_run) throw new Error('effect preview is stale')
      return result
    },
  })
  setTransport(mock)
  render(
    <LayerEffectsEditor
      doc={7}
      glyph={glyph}
      layer={layer}
      upem={1000}
      onClose={close}
      onApplied={vi.fn()}
    />,
  )
  const user = userEvent.setup()
  await user.click(screen.getByRole('button', { name: 'Add boolean mask' }))
  await user.click(screen.getByRole('button', { name: 'Capture mask' }))
  expect(
    await screen.findByDisplayValue('M0 0L40 0L40 40Z'),
  ).toBeInTheDocument()
  await user.click(screen.getByRole('button', { name: 'Preview changes' }))
  await screen.findByText(
    'Preview ready. Apply or bake this reviewed revision.',
  )
  await user.click(screen.getByRole('button', { name: 'Apply changes' }))
  expect(await screen.findByRole('alert')).toHaveTextContent('stale')
  expect(screen.getByRole('button', { name: 'Apply changes' })).toBeDisabled()
  expect(close).not.toHaveBeenCalled()
  await user.click(
    within(screen.getByRole('region', { name: '1. Boolean mask' })).getByRole(
      'checkbox',
    ),
  )
  await user.click(screen.getByRole('button', { name: 'Preview changes' }))
  await screen.findByText(
    'Preview ready. Apply or bake this reviewed revision.',
  )
  expect(
    mock.calls.filter((c) => c.method === 'effects.apply').at(-1)?.params,
  ).toMatchObject({ effects: [{ enabled: false }] })
})
