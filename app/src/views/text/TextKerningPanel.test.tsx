import { fireEvent, render, screen } from '@testing-library/react'
import { beforeAll, describe, expect, it, vi } from 'vitest'
import { i18n } from '../../i18n'
import en from '../../i18n/en.json'
import { TextKerningPanel } from './TextKerningPanel'
import type { TextKerningModel } from './kerningModel'

beforeAll(() => {
  i18n.addResourceBundle('en', 'translation', en, true, true)
})
function model(overrides: Partial<TextKerningModel> = {}): TextKerningModel {
  return {
    pair: { left: 'A', right: 'V' },
    scope: 'glyph_glyph',
    status: { editable: true, reason: null, generation: 1 },
    resolution: {
      status: { editable: true, reason: null, generation: 1 },
      master: 'm1',
      left: 'A',
      right: 'V',
      left_group: 'public.kern1.A',
      right_group: null,
      value: 0,
      rule: { left: { glyph: 'A' }, right: { glyph: 'V' }, value: 0 },
      records: [{ left: { glyph: 'A' }, right: { glyph: 'V' }, value: 0 }],
    },
    loading: false,
    loadFailed: false,
    editable: true,
    value: '0',
    invalid: false,
    editing: false,
    impact: [],
    impactLoading: false,
    impactFailed: false,
    setPair: vi.fn(async () => undefined),
    setScope: vi.fn(async () => undefined),
    setValue: vi.fn(),
    commit: vi.fn(async () => undefined),
    cancel: vi.fn(),
    remove: vi.fn(async () => undefined),
    ...overrides,
  }
}

describe('TextKerningPanel', () => {
  it('shows explicit zero, winning source, and only available scopes', () => {
    render(<TextKerningPanel model={model()} />)
    expect(screen.getByTestId('text-kerning-value')).toHaveValue('0')
    expect(screen.getByText('Stored at this scope: 0')).toBeInTheDocument()
    expect(screen.getByText('From A / V')).toBeInTheDocument()
    expect(screen.getByRole('option', { name: 'Group / glyph' })).toBeEnabled()
    expect(screen.getByRole('option', { name: 'Glyph / group' })).toBeDisabled()
    expect(screen.getByTestId('text-kerning-remove')).toBeEnabled()
  })

  it('cancels glyph-name Escape without committing the blur draft', () => {
    const data = model()
    render(<TextKerningPanel model={data} />)
    const field = screen.getByTestId('text-kerning-first')
    field.focus()
    fireEvent.change(field, { target: { value: 'T' } })
    fireEvent.keyDown(field, { key: 'Escape' })
    expect(field).toHaveValue('A')
    expect(data.setPair).not.toHaveBeenCalled()
  })

  it('preserves a newer second-name draft while the prior pair commit arrives', () => {
    const data = model()
    const view = render(<TextKerningPanel model={data} />)
    const first = screen.getByTestId('text-kerning-first')
    const second = screen.getByTestId('text-kerning-second')
    fireEvent.change(first, { target: { value: 'T' } })
    fireEvent.blur(first)
    fireEvent.change(second, { target: { value: 'o' } })
    view.rerender(
      <TextKerningPanel model={{ ...data, pair: { left: 'T', right: 'V' } }} />,
    )
    expect(second).toHaveValue('o')
    fireEvent.blur(second)
    expect(data.setPair).toHaveBeenLastCalledWith({ left: 'T', right: 'o' })
  })

  it('routes numeric Enter/blur and Escape through the model and keeps removal separate', () => {
    const data = model()
    render(<TextKerningPanel model={data} />)
    const field = screen.getByTestId('text-kerning-value')
    fireEvent.change(field, { target: { value: '-80' } })
    expect(data.setValue).toHaveBeenCalledWith('-80')
    fireEvent.keyDown(field, { key: 'Enter' })
    expect(data.commit).toHaveBeenCalledOnce()
    fireEvent.keyDown(field, { key: 'Escape' })
    expect(data.cancel).toHaveBeenCalledOnce()
    fireEvent.click(screen.getByTestId('text-kerning-remove'))
    expect(data.remove).toHaveBeenCalledOnce()
  })

  it('retires old name drafts when selection moves away and back', () => {
    const data = model()
    const view = render(<TextKerningPanel model={data} />)
    const first = screen.getByTestId('text-kerning-first')
    const second = screen.getByTestId('text-kerning-second')
    fireEvent.change(first, { target: { value: 'T' } })
    fireEvent.blur(first)
    fireEvent.change(second, { target: { value: 'o' } })
    view.rerender(
      <TextKerningPanel model={{ ...data, pair: { left: 'T', right: 'V' } }} />,
    )
    expect(second).toHaveValue('o')
    view.rerender(
      <TextKerningPanel model={{ ...data, pair: { left: 'W', right: 'a' } }} />,
    )
    view.rerender(
      <TextKerningPanel model={{ ...data, pair: { left: 'T', right: 'V' } }} />,
    )
    expect(second).toHaveValue('V')
  })

  it('explains unsupported ownership while preserving explicit pair controls', () => {
    const data = model({
      editable: false,
      resolution: null,
      status: { editable: false, reason: 'imported_layout', generation: 1 },
    })
    render(
      <TextKerningPanel
        model={data}
        gapDisabledReason="Ambiguous shaped cluster"
      />,
    )
    expect(screen.getByTestId('text-kerning-value')).toBeDisabled()
    expect(screen.getByTestId('text-kerning-first')).toBeEnabled()
    expect(
      screen.getByText(
        'The imported font’s spacing rules cannot yet be edited.',
      ),
    ).toBeInTheDocument()
    expect(screen.getByText('Ambiguous shaped cluster')).toBeInTheDocument()
  })
})
