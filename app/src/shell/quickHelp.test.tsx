import { act, fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { DocId } from '../api'
import { Dialog } from '../components/Dialog'
import { i18n } from '../i18n'
import { initialEditorState, useEditorStore } from '../stores/editorStore'
import { useKeymapStore } from '../stores/keymapStore'
import { initialUiState, useUiStore, type DialogState } from '../stores/uiStore'
import { useWorkspaceStore } from '../stores/workspaceStore'

import { ContextHelpDialog } from './ContextHelpDialog'
import {
  modalTopic,
  openContextHelp,
  QUICK_HELP_DELAY_MS,
  QUICK_HELP_LEAVE_MS,
  useContextHelpStore,
  useQuickHelpStore,
} from './contextHelp'
import { registerCommand, resetCommands } from './commands'

function openGlyph() {
  useUiStore.getState().openTab({ kind: 'glyph', doc: 1 as DocId })
}

beforeEach(() => {
  localStorage.clear()
  useKeymapStore.setState({ open: false, overrides: {} })
  useWorkspaceStore.setState({ dialogOpen: false })
  useUiStore.setState(initialUiState())
  useEditorStore.setState(initialEditorState())
  useContextHelpStore.getState().close()
  useQuickHelpStore.getState().hide()
  resetCommands()
})

afterEach(() => {
  vi.useRealTimers()
})

describe('quick help on hover', () => {
  it('waits for the hover delay before it appears', () => {
    vi.useFakeTimers()
    render(
      <>
        <button type="button" aria-description="Exports a font file.">
          Export
        </button>
        <ContextHelpDialog />
      </>,
    )
    fireEvent.pointerOver(screen.getByRole('button', { name: 'Export' }))
    act(() => void vi.advanceTimersByTime(QUICK_HELP_DELAY_MS - 1))
    expect(useQuickHelpStore.getState().target).toBeNull()
    act(() => void vi.advanceTimersByTime(1))
    expect(useQuickHelpStore.getState().target).toMatchObject({
      title: 'Export',
      summary: 'Exports a font file.',
    })
    expect(screen.getByText('Exports a font file.')).toBeVisible()
  })

  it('describes the command a control runs, with its live binding', () => {
    vi.useFakeTimers()
    registerCommand({
      id: 'file.export',
      title: 'cmd.file.export',
      shortcut: 'Mod+E',
      run: () => undefined,
    })
    useKeymapStore.setState({ open: false, overrides: { 'file.export': 'F7' } })
    render(
      <>
        <button type="button" data-help-command="file.export">
          Export…
        </button>
        <ContextHelpDialog />
      </>,
    )
    fireEvent.pointerOver(screen.getByRole('button', { name: 'Export…' }))
    act(() => void vi.advanceTimersByTime(QUICK_HELP_DELAY_MS))
    expect(useQuickHelpStore.getState().target).toMatchObject({
      topic: 'export',
      title: i18n.t('cmd.file.export'),
      summary: i18n.t('quickHelp.file.export'),
      binding: 'F7',
    })
  })

  it('says why a control cannot be used right now', () => {
    vi.useFakeTimers()
    registerCommand({
      id: 'file.save',
      title: 'cmd.file.save',
      when: () => false,
      disabledReason: 'cmd.reason.noDocument',
      run: () => undefined,
    })
    render(
      <>
        {/* An enabled element carries the marker: a disabled button receives
            no pointer events, and its reason is already shown inline. */}
        <span tabIndex={0} data-help-command="file.save">
          Save
        </span>
        <ContextHelpDialog />
      </>,
    )
    fireEvent.pointerOver(screen.getByText('Save'))
    act(() => void vi.advanceTimersByTime(QUICK_HELP_DELAY_MS))
    expect(useQuickHelpStore.getState().target?.reason).toBe(
      i18n.t('cmd.reason.noDocument'),
    )
  })

  it('opens the full article from a real click on its button', async () => {
    // `fireEvent.click` dispatches no `pointerdown`, so it cannot see a global
    // dismissal that unmounts the bubble before the click lands. A real pointer
    // sequence can, and did: this is the shape of the defect the review found.
    // Real timers: user-event's own pointer sequence is what has to run here.
    const user = userEvent.setup()
    render(
      <>
        <button type="button" data-help-topic="features">
          Features
        </button>
        <ContextHelpDialog />
      </>,
    )
    fireEvent.pointerOver(screen.getByRole('button', { name: 'Features' }))
    const open = await screen.findByRole(
      'button',
      { name: i18n.t('contextHelp.openArticle') },
      { timeout: QUICK_HELP_DELAY_MS + 2000 },
    )
    await user.click(open)
    expect(useQuickHelpStore.getState().target).toBeNull()
    expect(useContextHelpStore.getState().context?.topic).toBe('features')
    expect(
      screen.getByRole('heading', {
        name: i18n.t('contextHelp.topics.features.title'),
      }),
    ).toBeVisible()
  })

  it('survives a pointer press inside itself', () => {
    vi.useFakeTimers()
    render(
      <>
        <button type="button" data-help-topic="features">
          Features
        </button>
        <ContextHelpDialog />
      </>,
    )
    fireEvent.pointerOver(screen.getByRole('button', { name: 'Features' }))
    act(() => void vi.advanceTimersByTime(QUICK_HELP_DELAY_MS))
    const open = screen.getByRole('button', {
      name: i18n.t('contextHelp.openArticle'),
    })
    fireEvent.pointerDown(open)
    expect(useQuickHelpStore.getState().target).not.toBeNull()
    fireEvent.pointerOut(screen.getByRole('button', { name: 'Features' }), {
      relatedTarget: open,
    })
    act(() => void vi.advanceTimersByTime(QUICK_HELP_LEAVE_MS * 2))
    expect(useQuickHelpStore.getState().target).not.toBeNull()
  })

  it('hands focus to its button on Tab and activates it with Enter', async () => {
    const user = userEvent.setup()
    render(
      <>
        <button type="button" data-help-topic="features">
          Features
        </button>
        <ContextHelpDialog />
      </>,
    )
    const trigger = screen.getByRole('button', { name: 'Features' })
    act(() => trigger.focus())
    await screen.findByRole(
      'button',
      { name: i18n.t('contextHelp.openArticle') },
      { timeout: QUICK_HELP_DELAY_MS + 2000 },
    )
    expect(useQuickHelpStore.getState().target).not.toBeNull()

    await user.tab()
    const open = screen.getByRole('button', {
      name: i18n.t('contextHelp.openArticle'),
    })
    expect(open).toHaveFocus()
    expect(useQuickHelpStore.getState().target).not.toBeNull()

    await user.keyboard('{Enter}')
    expect(useContextHelpStore.getState().context?.topic).toBe('features')
  })

  it('keeps Tab inside itself from dismissing it, and Escape still does', () => {
    vi.useFakeTimers()
    render(
      <>
        <button type="button" data-help-topic="features">
          Features
        </button>
        <ContextHelpDialog />
      </>,
    )
    const trigger = screen.getByRole('button', { name: 'Features' })
    act(() => trigger.focus())
    act(() => void vi.advanceTimersByTime(QUICK_HELP_DELAY_MS))
    const bubble = document.querySelector('[data-quick-help]')
    expect(bubble).not.toBeNull()
    fireEvent.keyDown(bubble as Element, { key: 'Tab' })
    expect(useQuickHelpStore.getState().target).not.toBeNull()
    fireEvent.keyDown(bubble as Element, { key: 'Escape' })
    expect(useQuickHelpStore.getState().target).toBeNull()
  })

  it('says a control name once when its description repeats it', () => {
    vi.useFakeTimers()
    render(
      <>
        <button
          type="button"
          aria-label="Toggle sidebar"
          title="Toggle sidebar"
          data-help-topic="workspace"
        />
        <ContextHelpDialog />
      </>,
    )
    fireEvent.pointerOver(
      screen.getByRole('button', { name: 'Toggle sidebar' }),
    )
    act(() => void vi.advanceTimersByTime(QUICK_HELP_DELAY_MS))
    const target = useQuickHelpStore.getState().target
    expect(target?.title).toBe('Toggle sidebar')
    expect(target?.summary).toBe(i18n.t('contextHelp.topics.workspace.summary'))
    expect(screen.getAllByText('Toggle sidebar')).toHaveLength(1)
  })

  it('is dismissed by leaving, by a key press and by a pointer press', () => {
    vi.useFakeTimers()
    render(
      <>
        <button type="button" data-help-topic="export">
          Export
        </button>
        <ContextHelpDialog />
      </>,
    )
    const button = screen.getByRole('button', { name: 'Export' })
    const raise = () => {
      fireEvent.pointerOver(button)
      act(() => void vi.advanceTimersByTime(QUICK_HELP_DELAY_MS))
      expect(useQuickHelpStore.getState().target).not.toBeNull()
    }
    raise()
    fireEvent.pointerOut(button)
    // The dismissal waits out the grace period that lets the pointer reach the
    // bubble; nothing arrived, so it closes.
    expect(useQuickHelpStore.getState().target).not.toBeNull()
    act(() => void vi.advanceTimersByTime(QUICK_HELP_LEAVE_MS))
    expect(useQuickHelpStore.getState().target).toBeNull()
    raise()
    fireEvent.keyDown(document, { key: 'a' })
    expect(useQuickHelpStore.getState().target).toBeNull()
    raise()
    fireEvent.pointerDown(document)
    expect(useQuickHelpStore.getState().target).toBeNull()
  })

  it('stays away from a control that an open modal has covered', () => {
    vi.useFakeTimers()
    render(
      <>
        <button type="button" data-help-topic="export">
          Export
        </button>
        <Dialog title="Preferences" onCancel={() => undefined}>
          <button type="button" data-help-topic="preferences">
            Confirm
          </button>
        </Dialog>
        <ContextHelpDialog />
      </>,
    )
    fireEvent.pointerOver(screen.getByRole('button', { name: 'Export' }))
    act(() => void vi.advanceTimersByTime(QUICK_HELP_DELAY_MS))
    expect(useQuickHelpStore.getState().target).toBeNull()

    fireEvent.pointerOver(screen.getByRole('button', { name: 'Confirm' }))
    act(() => void vi.advanceTimersByTime(QUICK_HELP_DELAY_MS))
    expect(useQuickHelpStore.getState().target?.topic).toBe('preferences')
  })

  it('does not appear while the help article is open', () => {
    vi.useFakeTimers()
    render(
      <>
        <button type="button" data-help-topic="export">
          Export
        </button>
        <ContextHelpDialog />
      </>,
    )
    act(openContextHelp)
    fireEvent.pointerOver(screen.getByRole('button', { name: 'Export' }))
    act(() => void vi.advanceTimersByTime(QUICK_HELP_DELAY_MS))
    expect(useQuickHelpStore.getState().target).toBeNull()
  })

  it('also appears for a control that keyboard focus reaches', () => {
    vi.useFakeTimers()
    render(
      <>
        <button type="button" data-help-topic="shortcuts">
          Shortcuts
        </button>
        <ContextHelpDialog />
      </>,
    )
    act(() => screen.getByRole('button', { name: 'Shortcuts' }).focus())
    act(() => void vi.advanceTimersByTime(QUICK_HELP_DELAY_MS))
    expect(useQuickHelpStore.getState().target?.topic).toBe('shortcuts')
  })
})

describe('the help article follows the workspace', () => {
  it('names the active tool and its gesture hint', () => {
    openGlyph()
    useEditorStore.setState({ tool: 'measure' })
    act(openContextHelp)
    expect(useContextHelpStore.getState().context).toMatchObject({
      topic: 'glyph',
      toolTitle: i18n.t('tool.measure'),
      toolHint: i18n.t('tool.measureHint'),
    })
  })

  it('names a borrowed tool rather than the selected one', () => {
    openGlyph()
    useEditorStore.setState({ tool: 'select', borrowedTool: 'knife' })
    act(openContextHelp)
    expect(useContextHelpStore.getState().context?.toolTitle).toBe(
      i18n.t('tool.knife'),
    )
  })

  it('names no tool while the Edit tab is in shaped-text mode', () => {
    const tab = useUiStore
      .getState()
      .openTab({ kind: 'glyph', doc: 1 as DocId })
    useUiStore.getState().updateTab(tab, {
      textState: { mode: 'text' } as never,
    })
    useEditorStore.setState({ tool: 'measure' })
    act(openContextHelp)
    const context = useContextHelpStore.getState().context
    // The letters go into the run there, so no tool is in effect.
    expect(context?.topic).toBe('glyph')
    expect(context?.toolTitle).toBeUndefined()
  })

  it('describes the hovered command when F1 is pressed over a menu item', () => {
    registerCommand({
      id: 'path.removeOverlap',
      title: 'cmd.path.removeOverlap',
      shortcut: 'Mod+Shift+O',
      run: () => undefined,
    })
    render(
      <>
        <button type="button" data-help-command="path.removeOverlap">
          Remove Overlap
        </button>
        <ContextHelpDialog />
      </>,
    )
    fireEvent.pointerOver(
      screen.getByRole('button', { name: 'Remove Overlap' }),
    )
    act(openContextHelp)
    expect(useContextHelpStore.getState().context).toMatchObject({
      topic: 'outline',
      commandTitle: i18n.t('cmd.path.removeOverlap'),
      commandSummary: i18n.t('quickHelp.path.removeOverlap'),
    })
  })
})

describe('modal help topics', () => {
  const kinds: readonly DialogState['kind'][] = [
    'compareFonts',
    'pasteSpecial',
    'relinkComponents',
    'newFont',
    'batchRename',
    'addGlyphs',
    'export',
    'findProblems',
    'preferences',
    'recovery',
    'simplify',
    'expandStroke',
    'styleEffects',
  ]

  it.each(kinds)('maps the %s dialog to a specific article', (kind) => {
    useUiStore.setState({ dialog: { kind } as DialogState })
    const topic = modalTopic()
    expect(topic).not.toBe('dialog')
    expect(i18n.exists(`contextHelp.topics.${topic}.title`)).toBe(true)
  })

  it('maps the workspace manager and the shortcut editor', () => {
    useWorkspaceStore.setState({ dialogOpen: true })
    expect(modalTopic()).toBe('workspace')
    useKeymapStore.setState({ open: true })
    expect(modalTopic()).toBe('shortcuts')
  })

  it('falls back to the dialog article only when no dialog is identified', () => {
    expect(modalTopic()).toBe('dialog')
  })
})

it('does not repeat a command summary as a focused-control block', () => {
  registerCommand({
    id: 'file.export',
    title: 'cmd.file.export',
    shortcut: 'Mod+E',
    run: () => undefined,
  })
  render(
    <>
      <button
        type="button"
        data-help-command="file.export"
        aria-description={i18n.t('quickHelp.file.export')}
      >
        Export…
      </button>
      <ContextHelpDialog />
    </>,
  )
  fireEvent.pointerOver(screen.getByRole('button', { name: 'Export…' }))
  act(openContextHelp)
  const context = useContextHelpStore.getState().context
  expect(context?.commandSummary).toBe(i18n.t('quickHelp.file.export'))
  expect(context?.controlHint).toBeUndefined()
  expect(screen.getAllByText(i18n.t('quickHelp.file.export'))).toHaveLength(1)
})
