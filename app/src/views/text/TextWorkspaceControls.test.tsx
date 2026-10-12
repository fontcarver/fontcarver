import { useState, type ComponentProps } from 'react'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, expect, it, vi } from 'vitest'

import { initialUiState, useUiStore } from '../../stores/uiStore'
import { makeSummary } from '../../test/mockTransport'
import { initialTextState } from './textState'
import { TextWorkspaceControls } from './TextWorkspaceControls'

vi.mock('./TextBar', () => ({ TextBar: () => <input aria-label="Text" /> }))
vi.mock('./ProofLibraryPanel', () => ({
  ProofLibraryPanel: () => <button>Proof library</button>,
}))
vi.mock('./TextShapingControls', () => ({
  TextShapingControls: () => <button>Shaping controls</button>,
}))
vi.mock('./VariableProofPanel', () => ({ VariableProofPanel: () => null }))
vi.mock('./TextGapPicker', () => ({
  TextGapPicker: () => (
    <select aria-label="Kerning gap">
      <option>A / V</option>
    </select>
  ),
}))
vi.mock('./TextKerningPanel', () => ({
  TextKerningPanel: function DraftField() {
    const [value, setValue] = useState('')
    return (
      <input
        aria-label="Kerning draft"
        value={value}
        onChange={(event) => setValue(event.target.value)}
      />
    )
  },
}))
vi.mock('./KerningGhostControls', () => ({
  KerningGhostControls: () => (
    <input type="checkbox" aria-label="Group outlines" />
  ),
}))
vi.mock('./KerningTable', () => ({
  KerningTable: () => <button>Kerning table</button>,
}))
vi.mock('./SpacingMatrix', () => ({
  SpacingMatrix: () => <button>Spacing matrix</button>,
}))

type Props = ComponentProps<typeof TextWorkspaceControls>
function props(): Props {
  return {
    summary: makeSummary(),
    state: initialTextState('m1', 'A'),
    context: {},
    draft: 'A',
    inputRef: { current: null },
    glyphs: [],
    visible: [],
    pending: false,
    kerning: { selectedGap: null, gaps: [], panel: {} },
    ghosts: {},
    spacing: {},
    onTextChange: vi.fn(),
    onChange: vi.fn(),
    onSample: vi.fn(),
    onProofSelect: vi.fn(),
    proofId: null,
    stepRef: { current: null },
  } as unknown as Props
}

beforeEach(() => {
  localStorage.clear()
  useUiStore.setState(initialUiState())
})

it('keeps single-glyph kerning controls collapsed with a keyboard-accessible entry point', async () => {
  render(<TextWorkspaceControls {...props()} />)
  const toggle = screen.getByRole('button', { name: 'Kerning' })
  expect(toggle).toHaveAttribute('aria-expanded', 'false')
  expect(
    screen.queryByRole('combobox', { name: 'Kerning gap' }),
  ).not.toBeInTheDocument()
  expect(screen.getByRole('button', { name: 'Spacing matrix' })).toBeVisible()
  toggle.focus()
  await userEvent.setup().keyboard('{Enter}')
  expect(toggle).toHaveAttribute('aria-expanded', 'true')
  expect(screen.getByRole('combobox', { name: 'Kerning gap' })).toBeVisible()
})

it('remembers explicit panel preference without discarding a draft while collapsed', async () => {
  const user = userEvent.setup()
  const first = render(<TextWorkspaceControls {...props()} />)
  await user.click(screen.getByRole('button', { name: 'Kerning' }))
  await user.type(screen.getByRole('textbox', { name: 'Kerning draft' }), '-40')
  await user.click(screen.getByRole('button', { name: 'Kerning' }))
  expect(useUiStore.getState().sections['text.kerning']).toBe(false)
  await user.click(screen.getByRole('button', { name: 'Kerning' }))
  expect(screen.getByRole('textbox', { name: 'Kerning draft' })).toHaveValue(
    '-40',
  )
  first.unmount()
  render(<TextWorkspaceControls {...props()} />)
  expect(screen.getByRole('button', { name: 'Kerning' })).toHaveAttribute(
    'aria-expanded',
    'true',
  )
})

it('opens for a newly selected canvas gap and does not reopen after an ordinary refresh', async () => {
  const initial = props()
  const view = render(<TextWorkspaceControls {...initial} />)
  const selected = {
    ...initial,
    kerning: { ...initial.kerning, selectedGap: 0 },
  }
  view.rerender(<TextWorkspaceControls {...selected} />)
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Kerning' })).toHaveAttribute(
      'aria-expanded',
      'true',
    ),
  )
  await userEvent.setup().click(screen.getByRole('button', { name: 'Kerning' }))
  view.rerender(<TextWorkspaceControls {...selected} pending />)
  expect(screen.getByRole('button', { name: 'Kerning' })).toHaveAttribute(
    'aria-expanded',
    'false',
  )
  view.rerender(
    <TextWorkspaceControls
      {...selected}
      kerning={{ ...selected.kerning, selectedGap: 1 }}
    />,
  )
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Kerning' })).toHaveAttribute(
      'aria-expanded',
      'true',
    ),
  )
})

it('introduces kerning in a new text run while respecting an explicit collapsed preference', () => {
  const initial = props()
  initial.state = { ...initial.state, mode: 'text' }
  initial.kerning.gaps = [{}] as Props['kerning']['gaps']
  const view = render(<TextWorkspaceControls {...initial} />)
  expect(screen.getByRole('button', { name: 'Kerning' })).toHaveAttribute(
    'aria-expanded',
    'true',
  )
  view.unmount()
  useUiStore.getState().setSection('text.kerning', false)
  render(<TextWorkspaceControls {...initial} />)
  expect(screen.getByRole('button', { name: 'Kerning' })).toHaveAttribute(
    'aria-expanded',
    'false',
  )
})
