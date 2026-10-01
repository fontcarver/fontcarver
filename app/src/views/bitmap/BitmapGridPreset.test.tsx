import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { BitmapGridChoice, BitmapGridInfo } from '../../api'
import { BitmapGridPreset } from './BitmapGridPreset'

const call = vi.hoisted(() => vi.fn())
vi.mock('../../api', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../api')>()),
  call,
}))

function choice(ppem: number, upm = 1000): BitmapGridChoice {
  return {
    ppem,
    units_per_pixel: upm / ppem,
    exact: upm % ppem === 0,
    ascent: Math.round(ppem * 0.8),
    descent: Math.round(ppem * 0.2),
  }
}

function info(partial: Partial<BitmapGridInfo> = {}): BitmapGridInfo {
  return {
    generation: 3,
    units_per_em: 1000,
    presets: [4, 5, 8, 10, 20, 25, 40, 50, 100, 125, 200, 250, 500, 1000].map(
      (ppem) => choice(ppem),
    ),
    requested: null,
    nearest: null,
    ...partial,
  }
}

beforeEach(() => {
  call.mockReset()
})

describe('exact bitmap grid presets', () => {
  it('lists every exact grid the kernel reports and emits its metrics', async () => {
    call.mockResolvedValue(info({ requested: choice(20) }))
    const onSelect = vi.fn()
    render(
      <BitmapGridPreset doc={1} value={20} master="m" onSelect={onSelect} />,
    )
    const select = await screen.findByRole('combobox', {
      name: 'Integer pixel grid',
    })
    expect(call).toHaveBeenCalledWith('bitmap.grid', {
      doc: 1,
      master: 'm',
      ppem: 20,
    })
    await waitFor(() => expect(select).toHaveValue('20'))
    // Every offered size divides the units per em exactly.
    const options = screen
      .getAllByRole('option')
      .map((option) => (option as HTMLOptionElement).value)
      .filter(Boolean)
    expect(options).toHaveLength(14)
    for (const value of options) expect(1000 % Number(value)).toBe(0)
    fireEvent.change(select, { target: { value: '125' } })
    expect(onSelect).toHaveBeenCalledWith(choice(125))
  })

  it('reports a non-integer choice and offers the nearest exact size', async () => {
    call.mockResolvedValue(info({ requested: choice(16), nearest: 20 }))
    const onSelect = vi.fn()
    render(<BitmapGridPreset doc={1} value={16} onSelect={onSelect} />)
    expect(
      await screen.findByText(
        /62\.5 font units per pixel, which is not a whole/,
      ),
    ).toBeInTheDocument()
    expect(
      screen.getByRole('option', { name: /Custom \(16 ppem\)/ }),
    ).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Use 20 ppem instead' }))
    expect(onSelect).toHaveBeenCalledWith(choice(20))
  })

  it('confirms an exact choice with the metrics it would create', async () => {
    call.mockResolvedValue(info({ requested: choice(25) }))
    render(<BitmapGridPreset doc={1} value={25} onSelect={vi.fn()} />)
    expect(
      await screen.findByText(
        /exactly 40 font units per pixel; a new strike takes ascent 20 and descent 5/,
      ),
    ).toBeInTheDocument()
    expect(
      screen.queryByRole('button', { name: /instead/ }),
    ).not.toBeInTheDocument()
  })

  it('disables the control when the font has no exact grid', async () => {
    call.mockResolvedValue(info({ presets: [], requested: choice(16, 1001) }))
    render(<BitmapGridPreset doc={1} value={16} onSelect={vi.fn()} />)
    await waitFor(() => expect(screen.getByRole('combobox')).toBeDisabled())
  })
})
