import { fireEvent, render, screen } from '@testing-library/react'
import { expect, it, vi } from 'vitest'

import { PreviewStrip } from './PreviewStrip'
import type { PreviewStripProps } from './presentationTypes'

function props(height?: number): PreviewStripProps {
  return {
    glyphs: [],
    unitsPerEm: 1000,
    status: 'ready',
    theme: 'light',
    collapsed: false,
    height,
    onHeightChange: vi.fn(),
    onDismissStatus: vi.fn(),
    onThemeChange: vi.fn(),
    onCollapsedChange: vi.fn(),
    onSettingsChange: vi.fn(),
    settings: {
      flipHorizontal: false,
      flipVertical: false,
      blur: 0,
      mode: 'run',
      sizes: [12, 24, 48],
      instance: 'master:regular',
    },
  }
}

it.each([undefined, 120, 248])(
  'shows the default or saved height %s and expands settings without saving a new height',
  (height) => {
    const config = props(height)
    render(<PreviewStrip {...config} />)
    const resize = screen.getByRole('separator', { name: 'Resize preview' })
    const settings = screen.getByRole('button', { name: 'Preview settings' })
    expect(resize).toHaveAttribute('aria-valuenow', String(height ?? 96))
    expect(settings).toHaveAttribute('aria-expanded', 'false')
    expect(
      screen.queryByRole('spinbutton', { name: 'Blur (px)' }),
    ).not.toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: 'Collapse preview' }),
    ).toBeVisible()
    expect(screen.getAllByRole('img')).toHaveLength(3)
    fireEvent.click(settings)
    expect(resize).toHaveAttribute(
      'aria-valuenow',
      String(Math.max(160, height ?? 96)),
    )
    expect(screen.getByRole('spinbutton', { name: 'Blur (px)' })).toBeVisible()
    fireEvent.change(
      screen.getByRole('textbox', { name: 'Waterfall sizes (px)' }),
      { target: { value: '16, 32' } },
    )
    fireEvent.click(settings)
    expect(resize).toHaveAttribute('aria-valuenow', String(height ?? 96))
    fireEvent.click(settings)
    expect(
      screen.getByRole('textbox', { name: 'Waterfall sizes (px)' }),
    ).toHaveValue('16, 32')
    expect(config.onHeightChange).not.toHaveBeenCalled()
  },
)
