import { fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import {
  createRecordingContext,
  type RecordingContext,
} from '../../test/recordingContext'

import { PreviewStrip } from './PreviewStrip'
import type { PreviewStripProps } from './presentationTypes'

const contexts: RecordingContext[] = []

function props(overrides: Partial<PreviewStripProps> = {}): PreviewStripProps {
  return {
    glyphs: [
      { path: 'M0 0L100 0L100 200Z', x: 10, y: 20, xAdvance: 500, yAdvance: 0 },
      { path: 'M0 0L80 0L80 180Z', x: 520, y: -10, xAdvance: 480, yAdvance: 0 },
    ],
    unitsPerEm: 1_000,
    status: 'ready',
    onDismissStatus: vi.fn(),
    theme: 'light',
    onThemeChange: vi.fn(),
    collapsed: false,
    onCollapsedChange: vi.fn(),
    ...overrides,
  }
}

afterEach(() => {
  vi.restoreAllMocks()
  contexts.length = 0
})

describe('PreviewStrip', () => {
  it('renders each positioned path at an independent UPEM scale with y-up canvas transforms', () => {
    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockImplementation(
      () => {
        const recording = createRecordingContext()
        contexts.push(recording)
        return recording.ctx
      },
    )

    render(<PreviewStrip {...props()} />)

    expect(contexts).toHaveLength(3)
    expect(
      contexts.map(({ calls }) => calls('setTransform')[1]?.args[0]),
    ).toEqual([0.012, 0.024, 0.048])
    for (const recording of contexts) {
      expect(recording.calls('setTransform')[1]?.args[3]).toBeLessThan(0)
      expect(recording.calls('translate').map((call) => call.args)).toEqual([
        [10, 20],
        [520, -10],
      ])
      expect(recording.calls('fill')).toHaveLength(2)
      expect(recording.calls('fillText')).toHaveLength(0)
    }
  })

  it('keeps strip theme independent and forwards theme, collapse, and status dismissal', () => {
    const callbacks = props({ status: 'stale', theme: 'dark' })
    const { container, rerender } = render(<PreviewStrip {...callbacks} />)

    expect(container.firstElementChild).toHaveAttribute(
      'data-preview-theme',
      'dark',
    )
    expect(getComputedStyle(screen.getByRole('status')).color).toBe(
      getComputedStyle(container.firstElementChild as Element).color,
    )
    expect(screen.getByText('Recompiling…')).toBeVisible()
    rerender(<PreviewStrip {...callbacks} status="compile-failed" />)
    expect(screen.getByText('Preview unavailable')).toBeVisible()
    fireEvent.click(screen.getByRole('button', { name: 'Use light preview' }))
    expect(callbacks.onThemeChange).toHaveBeenCalledWith('light')
    fireEvent.click(
      screen.getByRole('button', { name: 'Dismiss preview status' }),
    )
    expect(callbacks.onDismissStatus).toHaveBeenCalledOnce()
    fireEvent.click(screen.getByRole('button', { name: 'Collapse preview' }))
    expect(callbacks.onCollapsedChange).toHaveBeenCalledWith(true)
  })

  it('bounds the backing canvases for long runs and exposes horizontal scrolling', () => {
    const longRun = Array.from({ length: 500 }, (_, index) => ({
      path: 'M0 0L500 0L500 500Z',
      x: index * 1_000,
      y: 0,
      xAdvance: 1_000,
      yAdvance: 0,
    }))
    const { container } = render(
      <PreviewStrip {...props({ glyphs: longRun })} />,
    )

    const canvases = [...container.querySelectorAll('canvas')]
    expect(canvases).toHaveLength(3)
    expect(
      Math.max(...canvases.map((canvas) => canvas.width)),
    ).toBeLessThanOrEqual(4_096)
    expect(container.querySelectorAll('[data-preview-scroll]')).toHaveLength(3)
  })
})
