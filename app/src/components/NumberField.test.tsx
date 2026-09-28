import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

import { NumberField } from './NumberField'

describe('NumberField', () => {
  it('reconciles a rejected value and lets the same value be retried', async () => {
    const user = userEvent.setup()
    const onCommit = vi.fn()
    const { rerender } = render(
      <NumberField
        label="Radius"
        value={20}
        revision={1}
        onCommit={onCommit}
      />,
    )
    const field = screen.getByLabelText('Radius')
    await user.clear(field)
    await user.type(field, '800{Enter}')
    rerender(
      <NumberField
        label="Radius"
        value={20}
        revision={2}
        onCommit={onCommit}
      />,
    )
    expect(field).toHaveValue('20')
    await user.clear(field)
    await user.type(field, '800{Enter}')
    expect(onCommit).toHaveBeenCalledTimes(2)
  })

  it('commits once for an edit confirmed with Enter and then blurred', async () => {
    // Enter commits, and the blur that follows commits again — by then the
    // kernel round trip has not finished, so the field still holds the old
    // `value` and would send the very same command a second time, leaving two
    // undo entries for one edit.
    const user = userEvent.setup()
    const onCommit = vi.fn()
    render(<NumberField label="LSB" value={16} onCommit={onCommit} />)

    const field = screen.getByLabelText('LSB')
    await user.clear(field)
    await user.type(field, '200{Enter}')
    await user.tab()

    expect(onCommit).toHaveBeenCalledExactlyOnceWith(200)
  })

  it('commits again once the model has caught up', async () => {
    const user = userEvent.setup()
    const onCommit = vi.fn()
    const { rerender } = render(
      <NumberField label="LSB" value={16} onCommit={onCommit} />,
    )

    const field = screen.getByLabelText('LSB')
    await user.clear(field)
    await user.type(field, '200{Enter}')
    rerender(<NumberField label="LSB" value={200} onCommit={onCommit} />)

    await user.clear(field)
    await user.type(field, '16{Enter}')

    expect(onCommit).toHaveBeenNthCalledWith(1, 200)
    expect(onCommit).toHaveBeenNthCalledWith(2, 16)
  })

  it('shakes and writes nothing for an entry it cannot parse', async () => {
    const user = userEvent.setup()
    const onCommit = vi.fn()
    render(<NumberField label="LSB" value={16} onCommit={onCommit} />)

    const field = screen.getByLabelText('LSB')
    await user.clear(field)
    await user.type(field, 'abc{Enter}')

    expect(onCommit).not.toHaveBeenCalled()
    expect(field).toHaveValue('16')
  })
})
