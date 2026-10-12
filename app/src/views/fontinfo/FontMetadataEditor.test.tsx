import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, expect, it, vi } from 'vitest'
import { setTransport } from '../../api'
import { makeFontInfoView } from '../../test/fontFixtures'
import { createMockTransport, noChange } from '../../test/mockTransport'
import { FontMetadataEditor } from './FontMetadataEditor'

afterEach(() => setTransport(undefined))

function setup(fail = false) {
  const view = makeFontInfoView()
  const transport = createMockTransport({
    'doc.font_info': () => structuredClone(view),
    'doc.edit_begin': () => 72,
    'doc.edit_apply': ({ command }) => {
      if (fail) throw new Error('write failed')
      if ('SetFontInfo' in command) view.info = command.SetFontInfo.info
      return noChange()
    },
    'doc.edit_end': () => null,
  })
  setTransport(transport)
  const onClose = vi.fn()
  render(<FontMetadataEditor doc={982} onClose={onClose} />)
  return { onClose, transport, view }
}

it('labels metadata fields, explains UPM, and applies edits on Close', async () => {
  const test = setup()
  const family = await screen.findByLabelText('Family name')
  expect(screen.getByText(/does not scale outlines/)).toBeInTheDocument()
  fireEvent.change(family, { target: { value: 'New Family' } })
  fireEvent.change(screen.getByLabelText('Units per em'), {
    target: { value: '1000' },
  })
  await userEvent.click(screen.getByRole('button', { name: 'Close' }))
  await waitFor(() => expect(test.onClose).toHaveBeenCalledOnce())
  expect(test.view.info.family_name).toBe('New Family')
  expect(test.view.info.units_per_em).toBe(1000)
})

it('keeps invalid and rejected drafts visible without closing', async () => {
  const test = setup(true)
  fireEvent.change(await screen.findByLabelText('Family name'), {
    target: { value: 'Keep this' },
  })
  await userEvent.click(screen.getByRole('button', { name: 'Close' }))
  expect(await screen.findByRole('alert')).toHaveTextContent('write failed')
  expect(test.onClose).not.toHaveBeenCalled()
  expect(screen.getByLabelText('Family name')).toHaveValue('Keep this')
})

it('discards staged values without sending SetFontInfo', async () => {
  const test = setup()
  fireEvent.change(await screen.findByLabelText('Family name'), {
    target: { value: 'Discard this' },
  })
  await userEvent.click(
    screen.getByRole('button', { name: 'Discard and close' }),
  )
  await waitFor(() => expect(test.onClose).toHaveBeenCalledOnce())
  expect(
    test.transport.calls.some((call) => call.method === 'doc.edit_begin'),
  ).toBe(false)
})

it('keeps keyboard focus inside the dialog after applying a draft', async () => {
  const test = setup()
  fireEvent.change(await screen.findByLabelText('Family name'), {
    target: { value: 'Applied Family' },
  })
  await userEvent.click(screen.getByRole('button', { name: 'Apply changes' }))
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Close' })).toHaveFocus(),
  )
  expect(test.view.info.family_name).toBe('Applied Family')
  await userEvent.keyboard('{Escape}')
  await waitFor(() => expect(test.onClose).toHaveBeenCalledOnce())
})

it('keeps invalid numeric input on Escape without sending a command', async () => {
  const test = setup()
  const input = await screen.findByLabelText('Units per em')
  fireEvent.change(input, { target: { value: '1.5' } })
  fireEvent.keyDown(input, { key: 'Escape' })
  expect(await screen.findByRole('alert')).toHaveTextContent(
    'Enter a whole number',
  )
  expect(input).toHaveValue('1.5')
  expect(test.onClose).not.toHaveBeenCalled()
  expect(
    test.transport.calls.some((call) => call.method === 'doc.edit_begin'),
  ).toBe(false)
})
