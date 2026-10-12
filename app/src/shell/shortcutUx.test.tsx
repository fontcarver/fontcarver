import { useEffect } from 'react'
import { act, fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'

import {
  initialAutomationState,
  useAutomationStore,
} from '../automation/automationStore'
import { TOOLS } from '../editor/tools/registry'
import { initialEditorState, useEditorStore } from '../stores/editorStore'
import { useKeymapStore } from '../stores/keymapStore'
import { initialUiState, useUiStore } from '../stores/uiStore'
import {
  makeCell,
  makeGlyph,
  makeGlyphDetail,
  makeLayer,
  makeLayerOutline,
  makeMaster,
} from '../test/fontFixtures'
import { buildScene } from '../views/glyph/glyphScene'
import { registerToolCommands } from '../views/glyph/toolCommands'
import { useToolShortcuts } from '../views/glyph/useToolShortcuts'
import { CommandPalette } from './CommandPalette'
import { MenuBar } from './MenuBar'
import { registerCommand, resetCommands } from './commands'
import { keymapConflicts } from './keymap'
import {
  canonicalShortcut,
  parseKeymapDocument,
  serializeKeymap,
} from './keymapDocument'
import { keymapTargets } from './keymapTargets'
import {
  formatShortcut,
  installShortcuts,
  matchesShortcut,
  parseShortcut,
} from './shortcuts'

beforeEach(() => {
  resetCommands()
  useUiStore.setState(initialUiState())
  useEditorStore.setState(initialEditorState())
  useKeymapStore.setState({ overrides: {} })
  useAutomationStore.setState({ ...initialAutomationState(), loaded: true })
})

afterEach(() => {
  resetCommands()
  vi.useRealTimers()
})

it('lists one editable row per registered tool, with its actual default', () => {
  registerToolCommands()
  const targets = keymapTargets().filter((target) =>
    target.id.startsWith('tool.'),
  )
  expect(targets).toHaveLength(TOOLS.length)
  expect(targets.filter((target) => target.id === 'tool.pen')).toEqual([
    { id: 'tool.pen', title: 'tool.pen', shortcut: 'P', scope: 'glyph' },
  ])
})

function ToolHost() {
  useEffect(() => installShortcuts(), [])
  useToolShortcuts(true)
  return <canvas tabIndex={0} />
}

it('preserves customized tool tap and hold with the real global dispatcher installed', () => {
  vi.useFakeTimers()
  registerToolCommands()
  const newlyShippedDefault = vi.fn()
  registerCommand({
    id: 'test.newDefault',
    title: 'cmd.file.open',
    shortcut: 'U',
    run: newlyShippedDefault,
  })
  const layer = makeLayer()
  const glyph = makeGlyph({ layers: [layer] })
  useEditorStore.setState({
    activeGlyph: {
      doc: 1,
      gid: 42,
      glyph,
      cell: makeCell(),
      scene: buildScene({
        detail: makeGlyphDetail(glyph),
        cell: makeCell(),
        master: makeMaster(),
        masterId: 'm1',
        unitsPerEm: 1000,
      }),
      outline: makeLayerOutline(layer),
      master: 'm1',
      masterGuides: [],
      reload: () => undefined,
    },
  })
  useKeymapStore.setState({ overrides: { 'tool.pen': 'U' } })
  render(<ToolHost />)
  const canvas = document.querySelector('canvas')!
  canvas.focus()
  fireEvent.keyDown(canvas, { key: 'u', code: 'KeyU' })
  expect(useEditorStore.getState().tool).toBe('select')
  act(() => vi.advanceTimersByTime(250))
  expect(useEditorStore.getState().borrowedTool).toBe('pen')
  fireEvent.keyUp(canvas, { key: 'u', code: 'KeyU' })
  expect(useEditorStore.getState().tool).toBe('select')
  expect(useEditorStore.getState().borrowedTool).toBeNull()
  fireEvent.keyDown(canvas, { key: 'u', code: 'KeyU' })
  fireEvent.keyUp(canvas, { key: 'u', code: 'KeyU' })
  expect(useEditorStore.getState().tool).toBe('pen')
  expect(newlyShippedDefault).not.toHaveBeenCalled()
})

it.each([false, true])(
  'keeps physical Control distinct from Mod on macOS=%s',
  (mac) => {
    const control = parseShortcut('Control+Tab')
    expect(control).not.toBeNull()
    expect(
      matchesShortcut(
        new KeyboardEvent('keydown', { key: 'Tab', ctrlKey: true }),
        control!,
        mac,
      ),
    ).toBe(true)
    expect(
      matchesShortcut(
        new KeyboardEvent('keydown', { key: 'Tab', metaKey: true }),
        control!,
        mac,
      ),
    ).toBe(false)
    expect(formatShortcut('Control+Shift+Tab', mac)).toBe(
      mac ? '⌃⇧Tab' : 'Ctrl+Shift+Tab',
    )
    expect(parseShortcut('Ctrl+A')).toEqual(parseShortcut('Mod+A'))
    expect(parseShortcut('Cmd+A')).toEqual(parseShortcut('Mod+A'))
  },
)

it('preserves physical Control when normalizing and re-importing a custom binding', () => {
  expect(canonicalShortcut('control+shift+u')).toBe('Control+Shift+U')
  const bindings = { 'tool.pen': 'Control+Shift+U' }
  expect(parseKeymapDocument(serializeKeymap(bindings))).toEqual(bindings)
})

it('detects physical Control and Mod overlaps only on platforms where they coincide', () => {
  const targets = [
    { id: 'one', title: 'one', shortcut: 'Mod+U', scope: 'global' },
    { id: 'two', title: 'two', shortcut: 'Control+U', scope: 'global' },
  ]
  expect(keymapConflicts(targets, {}, false)).toEqual([
    { shortcut: 'Mod+U', ids: ['one', 'two'] },
  ])
  expect(keymapConflicts(targets, {}, true)).toEqual([])
})

it.each([{ isComposing: true }, { keyCode: 229 }])(
  'leaves IME confirmation and navigation to composition: %j',
  (composition) => {
    const run = vi.fn()
    registerCommand({ id: 'file.open', title: 'cmd.file.open', run })
    registerCommand({ id: 'file.save', title: 'cmd.file.save', run: vi.fn() })
    useUiStore.setState({ paletteOpen: true })
    render(<CommandPalette />)
    const input = screen.getByRole('combobox', { name: 'Commands' })
    const highlighted = input.getAttribute('aria-activedescendant')
    for (const key of ['ArrowDown', 'ArrowUp', 'Enter', 'Escape']) {
      expect(fireEvent.keyDown(input, { key, ...composition })).toBe(true)
      expect(input).toHaveAttribute('aria-activedescendant', highlighted)
      expect(useUiStore.getState().paletteOpen).toBe(true)
    }
    expect(run).not.toHaveBeenCalled()
    fireEvent.keyDown(input, { key: 'Enter' })
    expect(run).toHaveBeenCalledOnce()
  },
)

function menuFixture(run = vi.fn()) {
  registerCommand({
    id: 'file.open',
    title: 'cmd.file.open',
    menu: 'file',
    run,
  })
  registerCommand({
    id: 'file.save',
    title: 'cmd.file.save',
    menu: 'file',
    run: vi.fn(),
  })
  render(
    <>
      <MenuBar />
      <canvas tabIndex={0} />
      <input aria-label="Command target" />
    </>,
  )
  return document.querySelector<HTMLButtonElement>('[data-menu="file"]')!
}

it.each(['{Enter}', ' ', '{ArrowDown}'])(
  'enters a closed menu with %s',
  async (key) => {
    const title = menuFixture()
    title.focus()
    await userEvent.setup().keyboard(key)
    expect(screen.getByRole('menuitem', { name: /Open Font/ })).toHaveFocus()
  },
)

it('enters a menu already opened by pointer and supports reverse entry', async () => {
  const title = menuFixture()
  const user = userEvent.setup()
  await user.click(title)
  await user.keyboard('{ArrowDown}')
  expect(screen.getByRole('menuitem', { name: /Open Font/ })).toHaveFocus()
  await user.keyboard('{Escape}{ArrowUp}')
  expect(screen.getByRole('menuitem', { name: 'Save' })).toHaveFocus()
})

it('restores editor focus after a menu action so canvas keys continue working', async () => {
  const run = vi.fn()
  const title = menuFixture(run)
  const canvas = document.querySelector('canvas')!
  canvas.focus()
  const user = userEvent.setup()
  await user.click(title)
  await user.click(screen.getByRole('menuitem', { name: /Open Font/ }))
  expect(run).toHaveBeenCalledOnce()
  expect(canvas).toHaveFocus()
})

it('lets a menu command move focus to its own destination', async () => {
  const title = menuFixture(vi.fn(() => screen.getByRole('textbox').focus()))
  document.querySelector('canvas')!.focus()
  const user = userEvent.setup()
  await user.click(title)
  await user.click(screen.getByRole('menuitem', { name: /Open Font/ }))
  expect(screen.getByRole('textbox')).toHaveFocus()
})

it('dismisses a menu when Tab moves away from its items', async () => {
  const title = menuFixture()
  title.focus()
  const user = userEvent.setup()
  await user.keyboard('{ArrowDown}{Tab}')
  expect(screen.queryByRole('menu')).not.toBeInTheDocument()
  expect(document.querySelector('[data-menu="edit"]')).toHaveFocus()
})

it('dismisses a menu when focus leaves the menu bar', async () => {
  const title = menuFixture()
  await userEvent.setup().click(title)
  act(() => screen.getByRole('textbox').focus())
  expect(screen.queryByRole('menu')).not.toBeInTheDocument()
})
