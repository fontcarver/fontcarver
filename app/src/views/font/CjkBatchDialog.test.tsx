import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, expect, it, vi } from 'vitest'
import { setTransport } from '../../api'
import { createMockTransport, noChange } from '../../test/mockTransport'
import { CjkBatchDialog } from './CjkBatchDialog'
afterEach(() => setTransport(undefined))
it('applies only the reviewed selection and invalidates the preview when options change', async () => {
  const close = vi.fn()
  const mock = createMockTransport({
    'cjk.batch': (params) => {
      expect(params.glyphs).toEqual([3, 7])
      if (!params.dry_run) expect(params.expected_generation).toBe(12)
      return {
        generation: 12,
        glyph_count: 2,
        layer_count: 2,
        examples: [],
        change: params.dry_run ? null : noChange(),
      }
    },
  })
  setTransport(mock)
  render(
    <CjkBatchDialog
      doc={1}
      glyphs={[3, 7]}
      master="m"
      upem={1000}
      onClose={close}
    />,
  )
  const user = userEvent.setup()
  const apply = screen.getByRole('button', { name: 'Apply changes' })
  expect(apply).toBeDisabled()
  await user.click(screen.getByRole('button', { name: 'Preview changes' }))
  expect(await screen.findByRole('status')).toBeInTheDocument()
  expect(apply).toBeEnabled()
  await user.click(screen.getByLabelText('Set vertical metrics'))
  expect(apply).toBeDisabled()
  await user.click(screen.getByRole('button', { name: 'Preview changes' }))
  await screen.findByRole('status')
  await user.click(apply)
  expect(close).toHaveBeenCalledOnce()
  expect(mock.calls.filter((c) => c.method === 'cjk.batch')).toHaveLength(3)
})
