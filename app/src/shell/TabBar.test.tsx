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
