import { act, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it } from 'vitest'

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
})
