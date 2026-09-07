import { fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createRef } from 'react'
import { describe, expect, it, vi } from 'vitest'

import { TextBar, TEXT_SAMPLE_TEXT } from './TextBar'
import type { TextBarProps } from './presentationTypes'

function props(overrides: Partial<TextBarProps> = {}): TextBarProps {
  return {
    text: 'Hamburgefonstiv',
    onTextChange: vi.fn(),
    size: 48,
    onSizeChange: vi.fn(),
    masters: [
      { id: 'regular', name: 'Regular' },
      { id: 'bold', name: 'Bold' },
    ],
    master: 'regular',
    onMasterChange: vi.fn(),
    features: [
      ['kern', true],
      ['liga', false],
    ],
    onFeaturesChange: vi.fn(),
    direction: 'ltr',
    onDirectionChange: vi.fn(),
    onSample: vi.fn(),
    ...overrides,
  }
}

describe('TextBar', () => {
  it('forwards controlled text, master, feature, direction, and sample actions', async () => {
    const user = userEvent.setup()
    const inputRef = createRef<HTMLInputElement>()
    const callbacks = props({ inputRef })
    render(<TextBar {...callbacks} />)

    fireEvent.change(screen.getByRole('textbox', { name: 'Text' }), {
      target: { value: 'AV' },
    })
    expect(callbacks.onTextChange).toHaveBeenCalledWith('AV')
    expect(inputRef.current).toBe(screen.getByRole('textbox', { name: 'Text' }))

    await user.selectOptions(
      screen.getByRole('combobox', { name: 'Master' }),
      'bold',
    )
    expect(callbacks.onMasterChange).toHaveBeenCalledWith('bold')

    await user.click(screen.getByRole('button', { name: 'liga' }))
    expect(callbacks.onFeaturesChange).toHaveBeenCalledWith([
      ['kern', true],
      ['liga', true],
    ])

    await user.click(screen.getByRole('button', { name: 'Right to left' }))
    expect(callbacks.onDirectionChange).toHaveBeenCalledWith('rtl')

    await user.selectOptions(
      screen.getByRole('combobox', { name: 'Sample text' }),
      'chinese',
    )
    expect(callbacks.onSample).toHaveBeenCalledWith('chinese')
    expect(TEXT_SAMPLE_TEXT.chinese).toContain('天地')
    expect(TEXT_SAMPLE_TEXT.latin).toContain('quick brown fox')
    await user.selectOptions(
      screen.getByRole('combobox', { name: 'Sample text' }),
      'all-encoded',
    )
    expect(callbacks.onSample).toHaveBeenLastCalledWith('all-encoded')
  })

  it('keeps future sample and direction options disabled', () => {
    render(<TextBar {...props()} />)

    expect(
      screen.getByRole('option', { name: 'Kerning pairs — coming later' }),
    ).toBeDisabled()
    expect(
      screen.getByRole('button', { name: 'Vertical text — coming later' }),
    ).toBeDisabled()
  })

  it('cancels a focused size edit on Escape without committing on blur', async () => {
    const user = userEvent.setup()
    const onSizeChange = vi.fn()
    render(<TextBar {...props({ onSizeChange })} />)
    const size = screen.getByRole('textbox', { name: 'Preview size' })

    await user.click(size)
    await user.clear(size)
    await user.type(size, '72')
    expect(size).toHaveFocus()
    await user.keyboard('{Escape}')

    expect(size).not.toHaveFocus()
    expect(size).toHaveValue('48')
    expect(onSizeChange).not.toHaveBeenCalled()
  })

  it('rejects malformed sizes and clamps valid values to safe bounds', () => {
    const onSizeChange = vi.fn()
    render(<TextBar {...props({ onSizeChange })} />)
    const size = screen.getByRole('textbox', { name: 'Preview size' })

    fireEvent.change(size, { target: { value: 'large' } })
    fireEvent.blur(size)
    expect(onSizeChange).not.toHaveBeenCalled()
    expect(size).toHaveAttribute('aria-invalid', 'true')

    fireEvent.change(size, { target: { value: '9999' } })
    fireEvent.keyDown(size, { key: 'Enter' })
    expect(onSizeChange).toHaveBeenCalledWith(512)

    fireEvent.change(size, { target: { value: '0.2' } })
    fireEvent.keyDown(size, { key: 'Enter' })
    expect(onSizeChange).toHaveBeenLastCalledWith(1)
  })
})
