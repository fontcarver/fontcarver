import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { expect, it, vi } from 'vitest'
import { makeMaster } from '../../test/fontFixtures'
import { VirtualMastersEditor } from './VirtualMastersEditor'
import type { DesignSpaceValue } from './designSpaceSession'

const state: DesignSpaceValue = {
  generation: 0,
  axes: [
    {
      tag: 'wght',
      name: 'Weight',
      min: 100,
      default: 100,
      max: 900,
      map: [],
      hidden: false,
    },
  ],
  masters: [makeMaster({ location: { wght: 100 } })],
  instances: [],
  first_glyph: 'A',
  virtual_masters: [],
}

it('adds a named sparse coordinate without copying physical masters', async () => {
  const onChange = vi.fn()
  render(<VirtualMastersEditor state={state} onChange={onChange} />)
  await userEvent.click(
    screen.getByRole('button', { name: 'Add virtual master' }),
  )
  expect(onChange).toHaveBeenCalledWith([
    { name: 'Virtual master 1', location: { wght: 900 } },
  ])
  expect(state.masters).toHaveLength(1)
})

it('edits mapped design coordinates and removes a preset without changing glyph sources', async () => {
  const onChange = vi.fn()
  render(
    <VirtualMastersEditor
      state={{
        ...state,
        virtual_masters: [{ name: 'Heavy', location: { wght: 900 } }],
      }}
      onChange={onChange}
    />,
  )
  const user = userEvent.setup()
  const coordinate = screen.getByLabelText('wght')
  await user.clear(coordinate)
  await user.type(coordinate, '800{Enter}')
  expect(onChange).toHaveBeenLastCalledWith([
    { name: 'Heavy', location: { wght: 800 } },
  ])
  await user.click(
    screen.getByRole('button', { name: 'Remove virtual master' }),
  )
  expect(onChange).toHaveBeenLastCalledWith([])
})

it('requires a variation axis before allowing a virtual position', () => {
  render(
    <VirtualMastersEditor state={{ ...state, axes: [] }} onChange={vi.fn()} />,
  )
  expect(
    screen.getByRole('button', { name: 'Add virtual master' }),
  ).toBeDisabled()
})
