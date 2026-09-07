import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { setTransport, type ApplyParams } from '../../api'
import {
  createMockTransport,
  noChange,
  type MockTransport,
} from '../../test/mockTransport'
import { initialUiState, useUiStore } from '../../stores/uiStore'

import { ShapeDialog } from './ShapeDialog'

let transport: MockTransport

const target = { doc: 1, glyph: 'a', layer: 'layer-1' }

function applied(): ApplyParams[] {
  return transport.calls
    .filter((entry) => entry.method === 'doc.apply')
    .map((entry) => entry.params as ApplyParams)
}

beforeEach(() => {
  useUiStore.setState(initialUiState())
  transport = createMockTransport({ 'doc.apply': () => noChange([42]) })
  setTransport(transport)
})

afterEach(() => setTransport(undefined))

describe('ShapeDialog', () => {
  it('creates the contour a drag of the same box would have made', async () => {
    const user = userEvent.setup()
    let closed = 0
    render(
      <ShapeDialog
        request={{ kind: 'rectangle', at: { x: 40, y: 60 } }}
        target={target}
        curves="cubic"
        onClose={() => {
          closed += 1
        }}
        onCommitted={() => undefined}
      />,
    )

    expect(screen.getByRole('dialog', { name: 'New rectangle' })).toBeVisible()
    await user.click(screen.getByRole('button', { name: 'Create' }))

    const calls = applied()
    expect(calls).toHaveLength(1)
    expect(calls[0]?.label).toBe('Draw rectangle')
    const command = calls[0]?.command as {
      AddContour: { contour: { points: { x: number; y: number }[] } }
    }
    expect(
      command.AddContour.contour.points.map((point) => [point.x, point.y]),
    ).toEqual([
      [40, 60],
      [140, 60],
      [140, 160],
      [40, 160],
    ])
    expect(closed).toBe(1)
  })

  it('commits once even when Enter reaches the button and the dialog', async () => {
    const user = userEvent.setup()
    render(
      <ShapeDialog
        request={{ kind: 'ellipse', at: { x: 0, y: 0 } }}
        target={target}
        curves="cubic"
        onClose={() => undefined}
        onCommitted={() => undefined}
      />,
    )
    const create = screen.getByRole('button', { name: 'Create' })
    create.focus()
    await user.keyboard('{Enter}')
    expect(applied()).toHaveLength(1)
  })

  it('commits the latest field text when Enter confirms from that field', async () => {
    const user = userEvent.setup()
    render(
      <ShapeDialog
        request={{ kind: 'rectangle', at: { x: 40, y: 60 } }}
        target={target}
        curves="cubic"
        onClose={() => undefined}
        onCommitted={() => undefined}
      />,
    )

    const x = screen.getByRole('textbox', { name: 'X' })
    await user.clear(x)
    await user.type(x, '75{Enter}')

    const command = applied()[0]?.command as {
      AddContour: { contour: { points: { x: number; y: number }[] } }
    }
    expect(command.AddContour.contour.points[0]?.x).toBe(75)
  })

  it('traps Tab inside the dialog', async () => {
    const user = userEvent.setup()
    render(
      <ShapeDialog
        request={{ kind: 'rectangle', at: { x: 0, y: 0 } }}
        target={target}
        curves="cubic"
        onClose={() => undefined}
        onCommitted={() => undefined}
      />,
    )
    const dialog = screen.getByRole('dialog')
    // However far Tab is pressed, focus stays inside.
    for (let step = 0; step < 8; step += 1) {
      await user.tab()
      expect(dialog.contains(document.activeElement)).toBe(true)
    }
  })

  it('cancels on Escape', async () => {
    const user = userEvent.setup()
    let closed = 0
    render(
      <ShapeDialog
        request={{ kind: 'rectangle', at: { x: 0, y: 0 } }}
        target={target}
        curves="cubic"
        onClose={() => {
          closed += 1
        }}
        onCommitted={() => undefined}
      />,
    )
    await user.keyboard('{Escape}')
    expect(closed).toBe(1)
    expect(applied()).toHaveLength(0)
  })
})
