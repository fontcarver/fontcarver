import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, expect, it, vi } from 'vitest'
import { setTransport } from '../../api'
import { createMockTransport, noChange } from '../../test/mockTransport'
import { StyleEffectsDialog } from './StyleEffectsDialog'

afterEach(() => setTransport(undefined))

it('captures selection, invalidates changed options, and applies only the reviewed generation', async () => {
  const close = vi.fn()
  const mock = createMockTransport({
    'style.batch': (params) => {
      expect(params.glyphs).toEqual([3, 7])
      if (!params.dry_run) {
        expect(params.expected_generation).toBe(12)
        expect(params.options.scale).toEqual([0.75, 0.75])
        expect(params.scale_advance).toBe(true)
        expect(params.master).toBeNull()
      }
      return {
        generation: 12,
        glyph_count: 2,
        layer_count: 2,
        decomposed_layers: 1,
        approximated_layers: 1,
        approximated_sources: ['S / Regular'],
        examples: [],
        change: params.dry_run ? null : noChange(),
      }
    },
  })
  setTransport(mock)
  render(
    <StyleEffectsDialog
      doc={1}
      glyphs={[3, 7]}
      master="m"
      upem={1000}
      onClose={close}
    />,
  )
  const user = userEvent.setup()
  const apply = screen.getByRole('button', { name: 'Apply changes' })
  const preview = screen.getByRole('button', { name: 'Preview changes' })
  expect(apply).toBeDisabled()
  await user.click(preview)
  await screen.findByRole('status')
  expect(apply).toBeEnabled()
  expect(screen.getByRole('note')).toHaveTextContent(
    'polygon union and curve refitting',
  )
  await user.selectOptions(screen.getByRole('combobox'), 'smallCaps')
  expect(apply).toBeDisabled()
  await user.click(screen.getByLabelText('Apply to all masters'))
  await user.click(preview)
  await screen.findByRole('status')
  await user.click(apply)
  expect(close).toHaveBeenCalledOnce()
  expect(mock.calls.filter((c) => c.method === 'style.batch')).toHaveLength(3)
})

it('keeps the dialog open on stale apply and requires another preview', async () => {
  const close = vi.fn()
  const mock = createMockTransport({
    'style.batch': (params) => {
      if (!params.dry_run) throw new Error('style preview is stale')
      return {
        generation: 12,
        glyph_count: 1,
        layer_count: 1,
        decomposed_layers: 0,
        approximated_layers: 0,
        approximated_sources: [],
        examples: [],
        change: null,
      }
    },
  })
  setTransport(mock)
  render(
    <StyleEffectsDialog
      doc={1}
      glyphs={[3]}
      master="m"
      upem={1000}
      onClose={close}
    />,
  )
  const user = userEvent.setup()
  await user.clear(screen.getByLabelText('Weight X (units)'))
  await user.click(screen.getByRole('button', { name: 'Preview changes' }))
  expect(await screen.findByRole('alert')).toHaveTextContent('finite number')
  expect(mock.calls).toHaveLength(0)
  await user.type(screen.getByLabelText('Weight X (units)'), '20')
  await user.click(screen.getByRole('button', { name: 'Preview changes' }))
  await screen.findByRole('status')
  await user.click(screen.getByRole('button', { name: 'Apply changes' }))
  expect(await screen.findByRole('alert')).toHaveTextContent('stale')
  expect(screen.getByRole('button', { name: 'Apply changes' })).toBeDisabled()
  expect(close).not.toHaveBeenCalled()
})
