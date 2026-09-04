import { act, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it } from 'vitest'

import { initialEditorState, useEditorStore } from '../../stores/editorStore'

import { ToolPalette } from './ToolPalette'

beforeEach(() => useEditorStore.setState(initialEditorState()))

describe('ToolPalette', () => {
  it('renders every tool with its name and shortcut', () => {
    render(<ToolPalette />)
    expect(screen.getAllByRole('button')).toHaveLength(12)
    expect(screen.getByRole('button', { name: 'Select' })).toHaveAttribute(
      'title',
      'Select (V)',
    )
    expect(screen.getByRole('button', { name: 'Ellipse' })).toHaveAttribute(
      'title',
      'Ellipse (⇧F)',
    )
  })

  it('shows the active tool as pressed and switches on click', async () => {
    const user = userEvent.setup()
    render(<ToolPalette />)

    expect(screen.getByRole('button', { name: 'Select' })).toHaveAttribute(
      'aria-pressed',
      'true',
    )
    await user.click(screen.getByRole('button', { name: 'Hand' }))
    expect(useEditorStore.getState().tool).toBe('hand')
    expect(screen.getByRole('button', { name: 'Hand' })).toHaveAttribute(
      'aria-pressed',
      'true',
    )
  })

  it('shows a borrowed tool as active without changing the chosen one', () => {
    render(<ToolPalette />)
    act(() => useEditorStore.getState().borrowTool('zoom'))
    expect(screen.getByRole('button', { name: 'Zoom' })).toHaveAttribute(
      'aria-pressed',
      'true',
    )
    expect(useEditorStore.getState().tool).toBe('select')
  })
})
