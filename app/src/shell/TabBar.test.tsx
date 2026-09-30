import { act, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import * as editors from '../editor/activeEditor'
import type { EditorController } from '../editor/editorController'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { TabBar } from './TabBar'

beforeEach(() => {
  localStorage.clear()
  useUiStore.setState(initialUiState())
  useSessionStore.setState(initialSessionState())
})

afterEach(() => vi.restoreAllMocks())

it('keeps keyboard focus with the old tab until its edit finishes', async () => {
  useUiStore.getState().openTab({ kind: 'font', doc: 2, label: 'Second font' })
  const old = useUiStore
    .getState()
    .openTab({ kind: 'glyph', doc: 1, gid: 9, label: 'AV text' })
  let finish!: () => void
  const closing = new Promise<void>((resolve) => {
    finish = resolve
  })
  vi.spyOn(editors, 'activeEditor').mockReturnValue({
    documentId: 1,
    prepareTransition: () => closing,
  } as unknown as EditorController)
  render(<TabBar />)
  const active = screen.getByRole('tab', { name: 'AV text' })
  const next = screen.getByRole('tab', { name: 'Second font' })
  active.focus()
  fireEvent.keyDown(active, { key: 'ArrowLeft' })
  expect(active).toHaveFocus()
  expect(useUiStore.getState().activeTab).toBe(old)
  await act(async () => {
    finish()
    await closing
  })
  expect(next).toHaveFocus()
  expect(next).toHaveAttribute('aria-selected', 'true')
})

it('keeps Overview pinned while offering an explicit tear-off control for editing tabs', () => {
  useUiStore.getState().openTab({ kind: 'font', doc: 1, label: 'Overview' })
  useUiStore.getState().openTab({ kind: 'glyph', doc: 1, gid: 3, label: 'A' })
  render(<TabBar />)
  expect(
    screen.queryByRole('button', { name: 'Close Overview' }),
  ).not.toBeInTheDocument()
  expect(
    screen.getByRole('button', { name: 'Move A to a new window' }),
  ).toBeInTheDocument()
  expect(screen.getByRole('button', { name: 'Close A' })).toBeInTheDocument()
})

it('marks the pinned Overview and publishes each tab ordinal shortcut', () => {
  const ui = useUiStore.getState()
  ui.openTab({ kind: 'glyph', doc: 1, gid: 3, label: 'A' })
  ui.openTab({ kind: 'font', doc: 1, label: 'Overview' })
  ui.openTab({ kind: 'features', doc: 1 })
  render(<TabBar />)

  const tabs = screen.getAllByRole('tab')
  expect(tabs.map((tab) => tab.textContent)).toEqual([
    '⊙Overview',
    'A',
    'Features',
  ])
  expect(tabs[0]).toHaveAttribute('data-pinned', 'true')
  expect(tabs[0]).toHaveAttribute('title', 'Overview · Pinned')
  // The ordinals the keymap binds, in the strip's own order.
  expect(tabs[0]).toHaveAttribute('aria-keyshortcuts', 'Control+Alt+1')
  expect(tabs[1]).toHaveAttribute('aria-keyshortcuts', 'Control+Alt+2')
  expect(tabs[2]).toHaveAttribute('aria-keyshortcuts', 'Control+Alt+3')
  expect(tabs[1]).not.toHaveAttribute('data-pinned')
  // The pinned state is exposed, not left to a decorative icon, and it stays
  // out of the tab's accessible name.
  expect(tabs[0]).toHaveAccessibleDescription('Pinned')
  expect(tabs[0]).toHaveAccessibleName('Overview')
  expect(tabs[1]).not.toHaveAccessibleDescription('Pinned')
})

it('announces an ordinal only for the active document', () => {
  const ui = useUiStore.getState()
  ui.openTab({ kind: 'font', doc: 1, label: 'First' })
  ui.openTab({ kind: 'glyph', doc: 1, gid: 3, label: 'A' })
  const other = ui.openTab({ kind: 'font', doc: 2, label: 'Second' })
  ui.openTab({ kind: 'glyph', doc: 2, gid: 4, label: 'B' })
  useUiStore.getState().setActiveTab(other)
  render(<TabBar />)

  const named = (name: string) => screen.getByRole('tab', { name })
  // `Mod+Alt+<n>` resolves against the active document, so only its tabs may
  // advertise a binding; the other document's would select a different tab.
  expect(named('Second')).toHaveAttribute('aria-keyshortcuts', 'Control+Alt+1')
  expect(named('B')).toHaveAttribute('aria-keyshortcuts', 'Control+Alt+2')
  expect(named('First')).not.toHaveAttribute('aria-keyshortcuts')
  expect(named('A')).not.toHaveAttribute('aria-keyshortcuts')
})
