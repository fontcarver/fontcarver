import { fireEvent, render, screen } from '@testing-library/react'
import { expect, it, vi } from 'vitest'
import { KerningModeControl } from './KerningModeControl'

it('exposes all modes and forwards keyboard-accessible selection changes', () => {
  const onChange = vi.fn()
  render(<KerningModeControl mode="on" onChange={onChange} />)
  const selector = screen.getByRole('combobox', { name: 'Kerning mode' })
  expect(selector).toHaveValue('on')
  expect(screen.getAllByRole('option')).toHaveLength(3)
  fireEvent.change(selector, { target: { value: 'locked' } })
  expect(onChange).toHaveBeenCalledWith('locked')
})
