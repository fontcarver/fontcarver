import { act, fireEvent, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import type { DocId } from '../api'
import { Dialog } from '../components/Dialog'
import { i18n } from '../i18n'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { useKeymapStore } from '../stores/keymapStore'

import { getCommand, resetCommands, runCommand } from './commands'
import { ContextHelpDialog } from './ContextHelpDialog'
import {
  helpHint,
  openContextHelp,
  registerHelpCommands,
  useContextHelpStore,
} from './contextHelp'

beforeEach(() => {
  localStorage.clear()
  useKeymapStore.setState({ open: false, overrides: {} })
  useUiStore.setState(initialUiState())
  useContextHelpStore.getState().close()
  resetCommands()
})

describe('context help selection', () => {
  it.each(['font', 'glyph', 'bitmap', 'fontinfo', 'features'] as const)(
    'opens relevant help for the %s tab',
    (kind) => {
      useUiStore.getState().openTab({ kind, doc: 1 as DocId })
      openContextHelp()
      expect(useContextHelpStore.getState().context?.topic).toBe(kind)
    },
  )

  it('recognizes the shortcut editor without replacing its draft', () => {
    useKeymapStore.setState({ open: true, overrides: { 'file.open': 'Mod+K' } })
    render(
      <Dialog title="Keyboard Shortcuts" onCancel={() => undefined}>
        <input aria-label="Shortcut" />
      </Dialog>,
    )
    openContextHelp()
    expect(useContextHelpStore.getState().context?.topic).toBe('shortcuts')
    expect(useKeymapStore.getState().overrides).toEqual({
      'file.open': 'Mod+K',
    })
  })

  it('uses getting started help without an active document', () => {
    openContextHelp()
    expect(useContextHelpStore.getState().context?.topic).toBe('welcome')
  })

  it('gives the focused modal priority and keeps its accessible title', () => {
    useUiStore.getState().openTab({ kind: 'glyph', doc: 1 as DocId })
    useUiStore.getState().setDialog({ kind: 'export', doc: 1 as DocId })
    render(
      <Dialog title="Export font" onCancel={() => undefined}>
        <input aria-label="File name" title="Choose the output font path." />
      </Dialog>,
    )
    openContextHelp()
    expect(useContextHelpStore.getState().context).toEqual({
      topic: 'export',
      surfaceTitle: 'Export font',
      controlTitle: 'File name',
      controlHint: 'Choose the output font path.',
    })
    expect(useUiStore.getState().dialog?.kind).toBe('export')
  })

  it('honours an explicit help topic on a focused field', () => {
    render(
      <Dialog title="Custom workflow" onCancel={() => undefined}>
        <input aria-label="Lookup" {...helpHint('features')} />
      </Dialog>,
    )
    openContextHelp()
    expect(useContextHelpStore.getState().context?.topic).toBe('features')
    expect(useContextHelpStore.getState().context?.surfaceTitle).toBe(
      'Custom workflow',
    )
    expect(screen.getByLabelText('Lookup')).toHaveAttribute(
      'aria-description',
      i18n.t('contextHelp.topics.features.summary'),
    )
  })

  it('finds the last dialog when focus is temporarily outside it', () => {
    render(
      <>
        <div role="dialog" aria-label="Underlying dialog" />
        <div role="dialog" aria-labelledby="top-title">
          <h2 id="top-title">Active workflow</h2>
        </div>
      </>,
    )
    openContextHelp()
    expect(useContextHelpStore.getState().context).toMatchObject({
      topic: 'dialog',
      surfaceTitle: 'Active workflow',
    })
  })

  it('shows the hovered control topic when keyboard focus is outside a field', () => {
    render(
      <>
        <button type="button" {...helpHint('export')}>
          Export
        </button>
        <ContextHelpDialog />
      </>,
    )
    fireEvent.pointerOver(screen.getByRole('button', { name: 'Export' }))
    act(openContextHelp)
    expect(useContextHelpStore.getState().context?.topic).toBe('export')
    expect(useContextHelpStore.getState().context?.controlTitle).toBe('Export')
  })

  it('discards hover context when the pointer leaves or an input takes focus', () => {
    render(
      <>
        <button type="button" {...helpHint('export')}>
          Export
        </button>
        <input aria-label="Lookup" {...helpHint('features')} />
        <ContextHelpDialog />
      </>,
    )
    const button = screen.getByRole('button', { name: 'Export' })
    fireEvent.pointerOver(button)
    fireEvent.pointerOut(button)
    act(openContextHelp)
    expect(useContextHelpStore.getState().context?.topic).toBe('welcome')
    act(() => useContextHelpStore.getState().close())
    fireEvent.pointerOver(button)
    screen.getByLabelText('Lookup').focus()
    act(openContextHelp)
    expect(useContextHelpStore.getState().context?.topic).toBe('features')
  })

  it('ignores hovered background controls while a modal is active', () => {
    render(
      <>
        <button type="button" {...helpHint('export')}>
          Export
        </button>
        <Dialog title="Preferences" onCancel={() => undefined}>
          <button type="button">Confirm</button>
        </Dialog>
        <ContextHelpDialog />
      </>,
    )
    fireEvent.pointerOver(screen.getByRole('button', { name: 'Export' }))
    act(openContextHelp)
    expect(useContextHelpStore.getState().context?.topic).toBe('dialog')
  })

  it('registers a discoverable Help command usable inside text fields', () => {
    registerHelpCommands()
    expect(getCommand('help.context')).toMatchObject({
      menu: 'help',
      shortcut: 'F1',
      allowInInput: true,
      allowInModal: true,
    })
    expect(runCommand('help.context')).toBe(true)
    expect(useContextHelpStore.getState().context?.topic).toBe('welcome')
  })
})

describe('context help dialog', () => {
  it('keeps the underlying dialog open and restores focus after Escape', async () => {
    const user = userEvent.setup()
    const cancel = vi.fn()
    render(
      <>
        <Dialog title="Export font" onCancel={cancel}>
          <input aria-label="File name" />
        </Dialog>
        <ContextHelpDialog />
      </>,
    )
    const field = screen.getByLabelText('File name')
    field.focus()
    act(openContextHelp)
    expect(screen.getAllByRole('dialog')).toHaveLength(2)
    expect(useUiStore.getState().modalCount).toBe(2)
    expect(screen.getByText('Export font', { selector: 'p' })).toBeVisible()

    await user.keyboard('{Escape}')

    expect(screen.getAllByRole('dialog')).toHaveLength(1)
    expect(useUiStore.getState().modalCount).toBe(1)
    expect(field).toHaveFocus()
    expect(cancel).not.toHaveBeenCalled()
  })

  it('lets readers choose another topic without changing the active tab', async () => {
    const user = userEvent.setup()
    const tab = useUiStore.getState().openTab({
      kind: 'glyph',
      doc: 1 as DocId,
    })
    render(<ContextHelpDialog />)
    act(openContextHelp)

    const help = screen.getByRole('dialog')
    await user.selectOptions(
      within(help).getByRole('combobox', {
        name: i18n.t('contextHelp.topic'),
      }),
      'export',
    )
    expect(
      within(help).getByRole('heading', {
        name: i18n.t('contextHelp.topics.export.title'),
      }),
    ).toBeVisible()
    expect(useUiStore.getState().activeTab).toBe(tab)
  })

  it('ignores repeated opens and closes from its own close button', () => {
    render(<ContextHelpDialog />)
    act(openContextHelp)
    const context = useContextHelpStore.getState().context
    act(openContextHelp)
    expect(useContextHelpStore.getState().context).toBe(context)
    fireEvent.click(
      screen.getByRole('button', { name: i18n.t('contextHelp.close') }),
    )
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  })
})
