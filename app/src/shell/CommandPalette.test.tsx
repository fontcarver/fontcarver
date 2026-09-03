import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { initialUiState, useUiStore } from '../stores/uiStore'

import { CommandPalette } from './CommandPalette'
import { registerCommand, resetCommands } from './commands'

const openFont = vi.fn()
const saveAs = vi.fn()
const hidden = vi.fn()

beforeEach(() => {
  localStorage.clear()
  resetCommands()
  openFont.mockReset()
  saveAs.mockReset()
  hidden.mockReset()
  registerCommand({
    id: 'file.open',
    title: 'cmd.file.open',
    menu: 'file',
    shortcut: 'Mod+O',
    run: openFont,
  })
  registerCommand({
    id: 'file.saveAs',
    title: 'cmd.file.saveAs',
    menu: 'file',
    run: saveAs,
  })
  registerCommand({
    id: 'file.close',
    title: 'cmd.file.close',
    menu: 'file',
    when: () => false,
    run: hidden,
  })
  useUiStore.setState({ ...initialUiState(), paletteOpen: true })
})

afterEach(() => {
  resetCommands()
})

describe('CommandPalette', () => {
  it('lists enabled commands with their shortcut hints', () => {
    render(<CommandPalette />)

    expect(
      screen.getByRole('option', { name: /Open Font/ }),
    ).toBeInTheDocument()
    expect(screen.getByText('Ctrl+O')).toBeInTheDocument()
    expect(screen.queryByRole('option', { name: /Close Font/ })).toBeNull()
  })

  it('filters by title and runs the highlighted command on Enter', async () => {
    const user = userEvent.setup()
    render(<CommandPalette />)

    await user.type(screen.getByRole('combobox'), 'save')
    expect(screen.getAllByRole('option')).toHaveLength(1)

    await user.keyboard('{Enter}')

    expect(saveAs).toHaveBeenCalledOnce()
    expect(useUiStore.getState().paletteOpen).toBe(false)
  })

  it('moves the highlight with the arrow keys', async () => {
    const user = userEvent.setup()
    render(<CommandPalette />)

    await user.keyboard('{ArrowDown}{Enter}')

    expect(saveAs).toHaveBeenCalledOnce()
    expect(openFont).not.toHaveBeenCalled()
  })

  it('closes on Escape and returns focus to the previous element', async () => {
    const user = userEvent.setup()
    useUiStore.setState({ paletteOpen: false })
    const trigger = document.createElement('button')
    document.body.append(trigger)
    trigger.focus()

    const view = render(<CommandPalette />)
    useUiStore.setState({ paletteOpen: true })
    view.rerender(<CommandPalette />)

    await user.keyboard('{Escape}')
    view.rerender(<CommandPalette />)

    expect(useUiStore.getState().paletteOpen).toBe(false)
    expect(document.activeElement).toBe(trigger)
    trigger.remove()
  })

  it('reports when nothing matches', async () => {
    const user = userEvent.setup()
    render(<CommandPalette />)

    await user.type(screen.getByRole('combobox'), 'zzzzz')

    expect(screen.getByText('No matching commands')).toBeInTheDocument()
  })
})
