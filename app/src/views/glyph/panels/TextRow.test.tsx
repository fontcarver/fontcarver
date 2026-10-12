import { fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

import { TextRow } from './TextRow'

describe('TextRow', () => {
  it.each([false, true])(
    'cancels %s-line text on Escape without committing the blur',
    async (multiline) => {
      const user = userEvent.setup()
      const commit = vi.fn()
      render(
        <TextRow
          label="Name"
          value="A"
          multiline={multiline}
          onCommit={commit}
        />,
      )
      const field = screen.getByLabelText('Name')
      await user.clear(field)
      await user.type(field, 'A.changed')
      await user.keyboard('{Escape}')

      expect(commit).not.toHaveBeenCalled()
      expect(field).toHaveValue('A')
    },
  )

  it('selects the complete value when focused', () => {
    render(<TextRow label="Name" value="A.changed" onCommit={() => {}} />)
    const field = screen.getByLabelText<HTMLInputElement>('Name')
    fireEvent.focus(field)
    expect(field.selectionStart).toBe(0)
    expect(field.selectionEnd).toBe(field.value.length)
  })
})
