import { fireEvent, render, screen } from '@testing-library/react'
import { useState } from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import {
  createRecordingContext,
  type RecordingContext,
} from '../../test/recordingContext'

import { PreviewStrip } from './PreviewStrip'
import type { PreviewSettings, PreviewStripProps } from './presentationTypes'
import { PREVIEW_SIZES } from './previewPaint'

const contexts: RecordingContext[] = []

function settings(overrides: Partial<PreviewSettings> = {}): PreviewSettings {
  return {
    flipHorizontal: false,
    flipVertical: false,
    blur: 0,
    mode: 'run',
    sizes: [...PREVIEW_SIZES],
    instance: 'master:regular',
    ...overrides,
  }
}

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
    settings: settings(),
    onSettingsChange: vi.fn(),
    ...overrides,
  }
}

/**
 * The strip is fully controlled, so the harness holds its settings the way the
 * text view does: an interaction only reaches the canvas if the owner stores it.
 */
function Harness({
  overrides = {},
  onChange,
}: {
  overrides?: Partial<PreviewStripProps>
  onChange?: (changes: Partial<PreviewSettings>) => void
}) {
  const [current, setCurrent] = useState<PreviewSettings>(
    overrides.settings ?? settings(),
  )
  return (
    <PreviewStrip
      {...props(overrides)}
      settings={current}
      onSettingsChange={(changes) => {
        onChange?.(changes)
        setCurrent((previous) => ({ ...previous, ...changes }))
      }}
    />
  )
}

function recordContexts() {
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockImplementation(() => {
    const recording = createRecordingContext()
    contexts.push(recording)
    return recording.ctx
  })
}

afterEach(() => {
  vi.restoreAllMocks()
  contexts.length = 0
})

describe('PreviewStrip', () => {
  it('renders each positioned path at an independent UPEM scale with y-up canvas transforms', () => {
    recordContexts()

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

  it('reflects preview ink on both axes and applies bounded blur', () => {
    recordContexts()
    const onChange = vi.fn()
    render(<Harness onChange={onChange} />)
    fireEvent.click(
      screen.getByRole('button', { name: 'Flip preview horizontally' }),
    )
    expect(onChange).toHaveBeenLastCalledWith({ flipHorizontal: true })
    expect(contexts.at(-1)?.calls('setTransform')[1]?.args[0]).toBe(-0.048)
    fireEvent.click(
      screen.getByRole('button', { name: 'Flip preview vertically' }),
    )
    expect(contexts.at(-1)?.calls('setTransform')[1]?.args[3]).toBe(0.048)
    fireEvent.change(screen.getByRole('spinbutton', { name: 'Blur (px)' }), {
      target: { value: '1.5' },
    })
    expect(contexts.at(-1)?.styleAt('fill', 'filter')).toBe('blur(1.5px)')
    fireEvent.change(screen.getByRole('spinbutton', { name: 'Blur (px)' }), {
      target: { value: '1000' },
    })
    expect(onChange).toHaveBeenLastCalledWith({ blur: 4 })
    expect(contexts.at(-1)?.styleAt('fill', 'filter')).toBe('blur(4px)')
  })

  it('applies custom sizes and preserves rendered sizes after an invalid draft', () => {
    recordContexts()
    render(<Harness />)
    fireEvent.change(
      screen.getByRole('textbox', { name: 'Waterfall sizes (px)' }),
      {
        target: { value: '18, 72, 128' },
      },
    )
    fireEvent.click(screen.getByRole('button', { name: 'Apply sizes' }))
    expect(
      contexts.slice(-3).map(({ calls }) => calls('setTransform')[1]?.args[0]),
    ).toEqual([0.018, 0.072, 0.128])
    expect(
      screen
        .getAllByRole('img')
        .map((region) => region.getAttribute('aria-label')),
    ).toEqual(['18 px preview', '72 px preview', '128 px preview'])
    fireEvent.change(
      screen.getByRole('textbox', { name: 'Waterfall sizes (px)' }),
      {
        target: { value: '12, Infinity' },
      },
    )
    fireEvent.click(screen.getByRole('button', { name: 'Apply sizes' }))
    expect(screen.getByRole('alert')).toHaveTextContent(
      'Only numbers are allowed',
    )
    expect(screen.getAllByRole('img')).toHaveLength(3)
    expect(screen.getByRole('img', { name: '128 px preview' })).toBeVisible()
  })

  it('says which rule an entry broke instead of restating the grammar', () => {
    render(<Harness />)
    const field = screen.getByRole('textbox', {
      name: 'Waterfall sizes (px)',
    })
    const apply = screen.getByRole('button', { name: 'Apply sizes' })
    for (const [entry, message] of [
      ['', 'Enter at least one step'],
      ['12px', 'Only numbers are allowed'],
      ['4, 24', 'between 6 and 256'],
      ['24, 24', 'only once'],
      ['9,10,11,12,13,14,15,16,17', 'at most 8 steps'],
      ['48-12/4', 'smaller first size'],
    ] as const) {
      fireEvent.change(field, { target: { value: entry } })
      fireEvent.click(apply)
      expect(screen.getByRole('alert')).toHaveTextContent(message)
      expect(field).toHaveAttribute('aria-invalid', 'true')
    }
    // A corrected entry clears the message and the invalid state.
    fireEvent.change(field, { target: { value: '12, 24' } })
    fireEvent.click(apply)
    expect(screen.queryByRole('alert')).not.toBeInTheDocument()
    expect(field).toHaveAttribute('aria-invalid', 'false')
  })

  it('draws the first eight steps of a long range and says that it cut it', () => {
    const onChange = vi.fn()
    render(<Harness onChange={onChange} />)
    const field = screen.getByRole('textbox', {
      name: 'Waterfall sizes (px)',
    })
    // The hint's own example, which asks for nine steps.
    fireEvent.change(field, { target: { value: '8-72/8' } })
    fireEvent.click(screen.getByRole('button', { name: 'Apply sizes' }))
    expect(onChange).toHaveBeenLastCalledWith({
      sizes: [8, 16, 24, 32, 40, 48, 56, 64],
    })
    expect(field).toHaveValue('8, 16, 24, 32, 40, 48, 56, 64')
    expect(screen.getByRole('alert')).toHaveTextContent(
      'longer than the 8 steps a waterfall shows',
    )
    // The note is not a rejection: the field is valid and the steps are drawn.
    expect(field).toHaveAttribute('aria-invalid', 'false')
    expect(screen.getAllByRole('img')).toHaveLength(8)
  })

  it('expands a range with a step into labelled waterfall rows', () => {
    const onChange = vi.fn()
    render(<Harness onChange={onChange} />)
    const field = screen.getByRole('textbox', {
      name: 'Waterfall sizes (px)',
    })
    fireEvent.change(field, { target: { value: '8-32/8' } })
    fireEvent.click(screen.getByRole('button', { name: 'Apply sizes' }))
    expect(onChange).toHaveBeenLastCalledWith({ sizes: [8, 16, 24, 32] })
    expect(
      screen
        .getAllByRole('img')
        .map((region) => region.getAttribute('aria-label')),
    ).toEqual([
      '8 px preview',
      '16 px preview',
      '24 px preview',
      '32 px preview',
    ])
    expect(field).toHaveValue('8, 16, 24, 32')
  })

  it('shows only the active glyph at the origin in single-glyph mode', () => {
    recordContexts()
    const input = props()
    const { rerender } = render(
      <PreviewStrip
        {...input}
        settings={settings({ mode: 'single' })}
        activeGlyph={input.glyphs[1]}
      />,
    )
    expect(contexts.at(-1)?.calls('fill')).toHaveLength(1)
    expect(contexts.at(-1)?.calls('translate')[0]?.args).toEqual([0, 0])
    rerender(
      <PreviewStrip
        {...input}
        settings={settings({ mode: 'single' })}
        activeGlyph={null}
      />,
    )
    expect(contexts.at(-1)?.calls('fill')).toHaveLength(0)
  })

  it('asks the owner to switch content rather than keeping its own mode', () => {
    const onSettingsChange = vi.fn()
    render(<PreviewStrip {...props({ onSettingsChange })} />)
    fireEvent.change(
      screen.getByRole('combobox', { name: 'Preview content' }),
      { target: { value: 'single' } },
    )
    expect(onSettingsChange).toHaveBeenCalledWith({ mode: 'single' })
  })

  it('lists masters and named instances separately and reports the choice', () => {
    const onSettingsChange = vi.fn()
    render(
      <PreviewStrip
        {...props({
          onSettingsChange,
          instances: [
            { id: 'master:a', name: 'Regular', kind: 'master' },
            { id: 'master:b', name: 'Bold', kind: 'master' },
            { id: 'instance:0', name: 'SemiBold', kind: 'instance' },
          ],
          settings: settings({ instance: 'master:a' }),
        })}
      />,
    )
    const picker = screen.getByRole('combobox', { name: /instance/i })
    expect(
      [...picker.querySelectorAll('optgroup')].map((group) =>
        group.getAttribute('label'),
      ),
    ).toEqual(['Masters', 'Named instances'])
    fireEvent.change(picker, { target: { value: 'instance:0' } })
    expect(onSettingsChange).toHaveBeenCalledWith({ instance: 'instance:0' })
  })

  it('hides the instance picker when there is nothing to choose between', () => {
    render(
      <PreviewStrip
        {...props({
          instances: [{ id: 'master:a', name: 'Regular', kind: 'master' }],
        })}
      />,
    )
    expect(
      screen.queryByRole('combobox', { name: /instance/i }),
    ).not.toBeInTheDocument()
  })

  it('renders a vertical run on a taller canvas than a horizontal one', () => {
    const vertical = [
      {
        path: 'M0 0L100 0L100 200Z',
        x: 0,
        y: 0,
        xAdvance: 0,
        yAdvance: -1_000,
      },
      {
        path: 'M0 0L100 0L100 200Z',
        x: 0,
        y: -1_000,
        xAdvance: 0,
        yAdvance: -1_000,
      },
      {
        path: 'M0 0L100 0L100 200Z',
        x: 0,
        y: -2_000,
        xAdvance: 0,
        yAdvance: -1_000,
      },
    ]
    const { container } = render(
      <PreviewStrip
        {...props({ glyphs: vertical, settings: settings({ sizes: [48] }) })}
      />,
    )
    const canvas = container.querySelector('canvas')!
    expect(canvas.style.height).not.toBe('')
    expect(Number.parseFloat(canvas.style.height)).toBeGreaterThan(
      Number.parseFloat(canvas.style.width),
    )
  })

  it('resizes with pointer and keyboard input and routes detachment', () => {
    const onDetach = vi.fn()
    const onHeightChange = vi.fn()
    render(<PreviewStrip {...props({ onDetach, onHeightChange })} />)
    const divider = screen.getByRole('separator', { name: 'Resize preview' })
    expect(divider).toHaveAttribute('aria-valuenow', '160')
    fireEvent.pointerDown(divider, { pointerId: 1, clientY: 400, button: 0 })
    fireEvent.pointerMove(divider, { pointerId: 1, clientY: 360 })
    fireEvent.pointerUp(divider, { pointerId: 1 })
    expect(onHeightChange).toHaveBeenLastCalledWith(200)
    expect(divider).toHaveAttribute('aria-valuenow', '200')
    fireEvent.keyDown(divider, { key: 'Home' })
    expect(onHeightChange).toHaveBeenLastCalledWith(120)
    fireEvent.keyDown(divider, { key: 'ArrowDown' })
    expect(divider).toHaveAttribute('aria-valuenow', '120')
    fireEvent.keyDown(divider, { key: 'End' })
    expect(onHeightChange).toHaveBeenLastCalledWith(600)
    fireEvent.click(screen.getByRole('button', { name: 'Detach preview' }))
    expect(onDetach).toHaveBeenCalledOnce()
  })
})
