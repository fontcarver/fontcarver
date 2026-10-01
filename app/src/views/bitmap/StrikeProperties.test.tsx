import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { setTransport, type BitmapStrikeInfo } from '../../api'
import { createMockTransport, noChange } from '../../test/mockTransport'
import { StrikeProperties } from './StrikeProperties'

function strike(
  properties: BitmapStrikeInfo['properties'] = {},
): BitmapStrikeInfo {
  return {
    ppem: 16,
    depth: 1,
    ascent: 13,
    descent: 3,
    properties,
    glyph_count: 2,
  }
}

function mount(properties: BitmapStrikeInfo['properties'] = {}) {
  const configure = vi.fn(() => noChange())
  const current = strike(properties)
  setTransport(
    createMockTransport({
      'bitmap.state': () => ({ generation: 5, strikes: [current] }),
      'bitmap.configure': configure,
    }),
  )
  render(<StrikeProperties doc={900} strike={current} />)
  fireEvent.click(screen.getByText('Strike metrics and BDF properties'))
  return { configure }
}

afterEach(() => setTransport(undefined))

describe('the strike property editor', () => {
  it('adds a standard property with its declared type locked', () => {
    const { configure } = mount()
    const name = screen.getByLabelText('Property', { exact: true })
    fireEvent.change(name, { target: { value: 'RESOLUTION_X' } })
    const type = screen.getByLabelText('Value type')
    expect(type).toHaveValue('int')
    expect(type).toBeDisabled()
    fireEvent.change(screen.getByLabelText('Value', { exact: true }), {
      target: { value: '72' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Set property' }))
    return waitFor(() =>
      expect(configure).toHaveBeenCalledWith(
        expect.objectContaining({
          expected_generation: 5,
          properties: { RESOLUTION_X: { Int: 72 } },
        }),
      ),
    )
  })

  it('refuses a name or value the formats cannot store, without a request', () => {
    const { configure } = mount()
    const name = screen.getByLabelText('Property', { exact: true }),
      value = screen.getByLabelText('Value', { exact: true })
    fireEvent.change(name, { target: { value: 'MY NAME' } })
    fireEvent.change(value, { target: { value: 'x' } })
    fireEvent.click(screen.getByRole('button', { name: 'Set property' }))
    expect(screen.getByRole('alert').textContent).toContain('cannot contain')
    fireEvent.change(name, { target: { value: 'POINT_SIZE' } })
    fireEvent.change(value, { target: { value: 'eighty' } })
    fireEvent.click(screen.getByRole('button', { name: 'Set property' }))
    expect(screen.getByRole('alert').textContent).toContain('whole number')
    expect(configure).not.toHaveBeenCalled()
  })

  it('renames a custom property in one write and refuses a collision', async () => {
    const { configure } = mount({
      FOUNDRY: { Str: 'FontCarver' },
      MY_TAG: { Str: 'one' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Rename MY_TAG' }))
    const field = screen.getByLabelText('New name for MY_TAG')
    fireEvent.change(field, { target: { value: 'FOUNDRY' } })
    fireEvent.click(screen.getByRole('button', { name: 'Save name' }))
    expect(screen.getByRole('alert').textContent).toContain('already exists')
    expect(configure).not.toHaveBeenCalled()
    fireEvent.change(field, { target: { value: 'MY_OTHER_TAG' } })
    fireEvent.click(screen.getByRole('button', { name: 'Save name' }))
    await waitFor(() =>
      expect(configure).toHaveBeenCalledWith(
        expect.objectContaining({
          properties: {
            FOUNDRY: { Str: 'FontCarver' },
            MY_OTHER_TAG: { Str: 'one' },
          },
        }),
      ),
    )
  })

  it('edits a stored value in place and retypes only a custom property', async () => {
    const { configure } = mount({ MY_TAG: { Str: '12' } })
    fireEvent.click(
      screen.getByRole('button', { name: 'Edit the value of MY_TAG' }),
    )
    fireEvent.change(screen.getByLabelText('New value for MY_TAG'), {
      target: { value: '34' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Save value' }))
    await waitFor(() =>
      expect(configure).toHaveBeenCalledWith(
        expect.objectContaining({ properties: { MY_TAG: { Str: '34' } } }),
      ),
    )
    fireEvent.click(
      screen.getByRole('button', { name: 'Change the type of MY_TAG' }),
    )
    await waitFor(() =>
      expect(configure).toHaveBeenLastCalledWith(
        expect.objectContaining({ properties: { MY_TAG: { Int: 12 } } }),
      ),
    )
  })

  it('marks a property the writer derives and locks its type', () => {
    mount({ FONT_ASCENT: { Int: 13 } })
    expect(screen.getByText(/written from the strike/)).toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: 'Change the type of FONT_ASCENT' }),
    ).toBeDisabled()
  })

  it('removes a property without touching the others', async () => {
    const { configure } = mount({
      FOUNDRY: { Str: 'FontCarver' },
      MY_TAG: { Str: 'one' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Remove MY_TAG' }))
    await waitFor(() =>
      expect(configure).toHaveBeenCalledWith(
        expect.objectContaining({
          properties: { FOUNDRY: { Str: 'FontCarver' } },
        }),
      ),
    )
  })
})
