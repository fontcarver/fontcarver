import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { BitmapGridPreset, integerGridPresets } from './BitmapGridPreset'

describe('integer bitmap grid presets', () => {
  it.each([1000, 1024, 2048])(
    'uses only integer units per pixel at %i UPM',
    (upm) => {
      const presets = integerGridPresets(upm)
      expect(presets.length).toBeGreaterThan(2)
      for (const { ppem, unitsPerPixel } of presets) {
        expect(Number.isInteger(unitsPerPixel)).toBe(true)
        expect(ppem * unitsPerPixel).toBe(upm)
      }
      expect(presets.some(({ ppem }) => ppem === 8)).toBe(true)
    },
  )

  it('keeps a custom strike visible and emits an exact grid size', () => {
    const onSelect = vi.fn()
    render(
      <BitmapGridPreset unitsPerEm={1000} value={17} onSelect={onSelect} />,
    )
    const select = screen.getByRole('combobox', { name: 'Integer pixel grid' })
    expect(select).toHaveValue('17')
    expect(screen.getByRole('option', { name: /Custom.*17/ })).toBeVisible()
    fireEvent.change(select, { target: { value: '20' } })
    expect(onSelect).toHaveBeenCalledWith(20)
  })

  it('rejects invalid font units and disables unavailable grids', () => {
    expect(integerGridPresets(0)).toEqual([])
    expect(integerGridPresets(1000.5)).toEqual([])
    render(<BitmapGridPreset unitsPerEm={0} value={16} onSelect={vi.fn()} />)
    expect(screen.getByRole('combobox')).toBeDisabled()
  })
})
