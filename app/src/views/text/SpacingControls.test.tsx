import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

import { SpacingControls } from './SpacingControls'

describe('SpacingControls', () => {
  it('offers every side and forwards keyboard-accessible selection', async () => {
    const user = userEvent.setup()
    const onSideChange = vi.fn()
    render(<SpacingControls side="lsb" onSideChange={onSideChange} />)

    const select = screen.getByRole('combobox', { name: 'Spacing side' })
    expect(select).toHaveValue('lsb')
    expect(screen.getByRole('option', { name: 'LSB' })).toBeInTheDocument()
    expect(screen.getByRole('option', { name: 'RSB' })).toBeInTheDocument()
    expect(screen.getByRole('option', { name: 'Both' })).toBeInTheDocument()
    await user.tab()
    expect(select).toHaveFocus()
    await user.selectOptions(select, 'rsb')
    expect(onSideChange).toHaveBeenLastCalledWith('rsb')
    await user.selectOptions(select, 'both')
    expect(onSideChange).toHaveBeenLastCalledWith('both')
  })

  it('explains all four step modes and the true-one override', () => {
    render(<SpacingControls side="both" onSideChange={vi.fn()} />)

    const select = screen.getByRole('combobox', { name: 'Spacing side' })
    expect(select).toHaveAccessibleDescription(
      /Shift: ×10; Alt: ×100; Shift\+Alt: exactly 1 font unit/u,
    )
    expect(select).toHaveAccessibleDescription(/navigate between glyphs/u)
    expect(screen.getByText('[ / ]')).toBeInTheDocument()
    expect(select).toHaveValue('both')
  })

  it('disables side changes while spacing is unavailable', async () => {
    const onSideChange = vi.fn()
    render(<SpacingControls side="rsb" onSideChange={onSideChange} disabled />)

    const select = screen.getByRole('combobox', { name: 'Spacing side' })
    expect(select).toBeDisabled()
    await userEvent.selectOptions(select, 'lsb')
    expect(onSideChange).not.toHaveBeenCalled()
  })
})
