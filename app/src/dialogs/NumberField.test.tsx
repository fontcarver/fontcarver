import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

import { NumberField } from './NumberField'

async function typeAndBlur(
  bounds: { min?: number; max?: number },
  text: string,
): Promise<number | undefined> {
  const user = userEvent.setup()
  const onCommit = vi.fn()
  const { unmount } = render(
    <NumberField label="Value" value={1} {...bounds} onCommit={onCommit} />,
  )
  const field = screen.getByLabelText('Value')
  await user.clear(field)
  await user.type(field, text)
  await user.tab()
  unmount()
  return onCommit.mock.calls.at(-1)?.[0] as number | undefined
}

describe('NumberField bounds', () => {
  // A `type="number"` box with `min=0` still reports a typed −5, so the field
  // is the last guard before a preference reaches the kernel.
  it('clamps up to a minimum when there is no maximum', async () => {
    expect(await typeAndBlur({ min: 0 }, '-5')).toBe(0)
    expect(await typeAndBlur({ min: 0 }, '8')).toBe(8)
  })

  it('clamps down to a maximum when there is no minimum', async () => {
    expect(await typeAndBlur({ max: 10 }, '50')).toBe(10)
    expect(await typeAndBlur({ max: 10 }, '-50')).toBe(-50)
  })

  it('clamps to both bounds when both are given', async () => {
    expect(await typeAndBlur({ min: 0, max: 10 }, '50')).toBe(10)
    expect(await typeAndBlur({ min: 0, max: 10 }, '-5')).toBe(0)
    expect(await typeAndBlur({ min: 0, max: 10 }, '4')).toBe(4)
  })

  it('passes any finite value through when neither bound is given', async () => {
    expect(await typeAndBlur({}, '-5')).toBe(-5)
    expect(await typeAndBlur({}, '9999')).toBe(9999)
  })

  it('reports nothing for an emptied or unreadable box', async () => {
    const user = userEvent.setup()
    const onCommit = vi.fn()
    render(<NumberField label="Value" value={4} min={0} onCommit={onCommit} />)

    await user.clear(screen.getByLabelText('Value'))
    await user.tab()

    expect(onCommit).not.toHaveBeenCalled()
    expect(screen.getByLabelText('Value')).toHaveValue(4)
  })
})
