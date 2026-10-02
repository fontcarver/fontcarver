import { act, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { initialEditorState, useEditorStore } from '../../stores/editorStore'
import { useKeymapStore } from '../../stores/keymapStore'
import { formatShortcut } from '../../shell/shortcuts'
import { i18n } from '../../i18n'
import { TOOLS } from '../../editor/tools/registry'

import { ToolPalette } from './ToolPalette'

beforeEach(() => {
  useEditorStore.setState(initialEditorState())
  useKeymapStore.setState({ overrides: {} })
})

afterEach(() => vi.restoreAllMocks())

describe('ToolPalette', () => {
  it('renders every tool with its name and shortcut', () => {
    render(<ToolPalette />)
    expect(screen.getAllByRole('button')).toHaveLength(TOOLS.length)
    expect(screen.getByRole('button', { name: 'Select' })).toHaveAttribute(
      'title',
      `Select (V)\n${i18n.t('shortcuts.toolHint')}`,
    )
    expect(screen.getByRole('button', { name: 'Ellipse' })).toHaveAttribute(
      'title',
      `Ellipse (${formatShortcut('Shift+F')})\n${i18n.t('shortcuts.toolHint')}`,
    )
  })

  it('updates shortcut labels after remapping and hides disabled bindings', () => {
    render(<ToolPalette />)
    act(() =>
      useKeymapStore.setState({
        overrides: { 'tool.pen': 'Mod+Q', 'tool.ellipse': null },
      }),
    )
    expect(screen.getByRole('button', { name: 'Pen' })).toHaveAttribute(
      'title',
      `Pen (${formatShortcut('Mod+Q')})\n${i18n.t('shortcuts.toolHint')}`,
    )
    expect(screen.getByRole('button', { name: 'Ellipse' })).toHaveAttribute(
      'title',
      `Ellipse\n${i18n.t('shortcuts.toolHint')}`,
    )
    expect(screen.getByRole('button', { name: 'Pen' })).toHaveAttribute(
      'data-help-topic',
      'glyph',
    )
    expect(screen.getByRole('button', { name: 'Pen' })).toHaveAttribute(
      'aria-description',
      i18n.t('shortcuts.toolHint'),
    )
  })

  it('shows the active tool as pressed and switches on click', async () => {
    const user = userEvent.setup()
    render(<ToolPalette />)

    expect(screen.getByRole('button', { name: 'Select' })).toHaveAttribute(
      'aria-pressed',
      'true',
    )
    await user.click(screen.getByRole('button', { name: 'Hand' }))
    expect(useEditorStore.getState().tool).toBe('hand')
    expect(screen.getByRole('button', { name: 'Hand' })).toHaveAttribute(
      'aria-pressed',
      'true',
    )
  })

  it('shows a borrowed tool as active without changing the chosen one', () => {
    render(<ToolPalette />)
    act(() => useEditorStore.getState().borrowTool('zoom'))
    expect(screen.getByRole('button', { name: 'Zoom' })).toHaveAttribute(
      'aria-pressed',
      'true',
    )
    expect(useEditorStore.getState().tool).toBe('select')
  })

  it('reaches every tool with arrow keys and leaves the toolbar with Tab', async () => {
    const user = userEvent.setup()
    render(
      <>
        <button>Before tools</button>
        <ToolPalette />
        <button>After tools</button>
      </>,
    )
    await user.tab()
    await user.tab()
    expect(screen.getByRole('button', { name: 'Select' })).toHaveFocus()
    for (const tool of TOOLS.slice(1)) {
      await user.keyboard('{ArrowDown}')
      expect(
        screen.getByRole('button', { name: i18n.t(tool.labelKey) }),
      ).toHaveFocus()
    }
    expect(useEditorStore.getState().tool).toBe('select')
    await user.tab()
    expect(screen.getByRole('button', { name: 'After tools' })).toHaveFocus()
    await user.tab({ shift: true })
    expect(
      screen.getByRole('button', { name: i18n.t(TOOLS.at(-1)!.labelKey) }),
    ).toHaveFocus()
    await user.keyboard('{Home}')
    expect(screen.getByRole('button', { name: 'Select' })).toHaveFocus()
    await user.tab({ shift: true })
    expect(screen.getByRole('button', { name: 'Before tools' })).toHaveFocus()
  })

  it('keeps a focused or keyboard-selected lower tool in view without stealing canvas focus', async () => {
    const scroll = vi.fn()
    const original = HTMLElement.prototype.scrollIntoView
    HTMLElement.prototype.scrollIntoView = scroll
    try {
      const user = userEvent.setup()
      render(
        <>
          <button>Canvas focus</button>
          <ToolPalette />
        </>,
      )
      const canvas = screen.getByRole('button', { name: 'Canvas focus' })
      canvas.focus()
      act(() => useEditorStore.getState().setTool('hand'))
      expect(canvas).toHaveFocus()
      expect(scroll).toHaveBeenLastCalledWith({
        block: 'nearest',
        inline: 'nearest',
      })
      await user.tab()
      expect(screen.getByRole('button', { name: 'Hand' })).toHaveFocus()
      await user.keyboard('{End}')
      expect(
        screen.getByRole('button', { name: i18n.t(TOOLS.at(-1)!.labelKey) }),
      ).toHaveFocus()
      await user.keyboard('{Enter}')
      expect(useEditorStore.getState().tool).toBe(TOOLS.at(-1)!.id)
    } finally {
      HTMLElement.prototype.scrollIntoView = original
    }
  })
})
