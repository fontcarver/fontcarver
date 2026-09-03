import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

import { SplitPane } from './SplitPane'

function renderPane(side: 'left' | 'right', onResize = vi.fn()) {
  render(
    <SplitPane
      side={side}
      size={240}
      min={200}
      max={400}
      label="Resize sidebar"
      onResize={onResize}
    >
      <p>pane</p>
    </SplitPane>,
  )
  return { onResize, divider: screen.getByRole('separator') }
}

describe('SplitPane', () => {
  it('exposes the divider as a resizable separator', () => {
    const { divider } = renderPane('left')

    expect(divider).toHaveAttribute('aria-orientation', 'vertical')
    expect(divider).toHaveAttribute('aria-valuenow', '240')
    expect(divider).toHaveAttribute('aria-valuemin', '200')
    expect(divider).toHaveAttribute('aria-valuemax', '400')
  })

  it('resizes a left pane with the arrow keys, larger with Shift', async () => {
    const user = userEvent.setup()
    const { onResize, divider } = renderPane('left')
    divider.focus()

    await user.keyboard('{ArrowRight}')
    expect(onResize).toHaveBeenLastCalledWith(248)

    await user.keyboard('{ArrowLeft}')
    expect(onResize).toHaveBeenLastCalledWith(232)

    await user.keyboard('{Shift>}{ArrowRight}{/Shift}')
    expect(onResize).toHaveBeenLastCalledWith(272)
  })

  it('mirrors the arrow-key direction for a right pane', async () => {
    const user = userEvent.setup()
    const { onResize, divider } = renderPane('right')
    divider.focus()

    await user.keyboard('{ArrowRight}')
    expect(onResize).toHaveBeenLastCalledWith(232)
  })

  it('clamps to the configured bounds', async () => {
    const user = userEvent.setup()
    const onResize = vi.fn()
    render(
      <SplitPane
        side="left"
        size={398}
        min={200}
        max={400}
        label="Resize sidebar"
        onResize={onResize}
      >
        <p>pane</p>
      </SplitPane>,
    )
    screen.getByRole('separator').focus()

    await user.keyboard('{Shift>}{ArrowRight}{/Shift}')

    expect(onResize).toHaveBeenLastCalledWith(400)
  })

  it('ignores keys that are not horizontal arrows', async () => {
    const user = userEvent.setup()
    const { onResize, divider } = renderPane('left')
    divider.focus()

    await user.keyboard('{ArrowUp}{Enter}')

    expect(onResize).not.toHaveBeenCalled()
  })
})
