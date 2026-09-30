import { act, fireEvent, render, screen } from '@testing-library/react'
import { beforeEach, expect, it, vi } from 'vitest'

import { initialUiState, useUiStore } from '../stores/uiStore'
import {
  captureWorkspace,
  initialWorkspaceState,
  useWorkspaceStore,
} from '../stores/workspaceStore'

import { WorkspaceDialog } from './WorkspaceDialog'

beforeEach(() => {
  localStorage.clear()
  useUiStore.setState(initialUiState())
  useWorkspaceStore.setState(initialWorkspaceState())
})

it('saves, applies and deletes a named document workspace through labelled controls', async () => {
  const saved = captureWorkspace(1, initialUiState())
  const apply = vi.fn(() => true)
  render(
    <WorkspaceDialog
      documentKey="doc:1"
      capture={() => saved}
      onApply={apply}
      onClose={vi.fn()}
    />,
  )
  expect(screen.getByRole('button', { name: 'Save workspace' })).toBeDisabled()
  fireEvent.change(screen.getByRole('textbox', { name: 'Workspace name' }), {
    target: { value: 'Drawing' },
  })
  await act(async () => {
    fireEvent.click(screen.getByRole('button', { name: 'Save workspace' }))
  })
  await act(async () => {
    fireEvent.click(screen.getByRole('button', { name: 'Apply Drawing' }))
  })
  expect(apply).toHaveBeenCalledWith(saved)
  fireEvent.click(screen.getByRole('button', { name: 'Delete Drawing' }))
  expect(
    screen.getByText('No saved workspaces for this document.'),
  ).toBeVisible()
})

it('preserves the current workspace when applying a preset cannot finish edits', async () => {
  const saved = captureWorkspace(1, initialUiState())
  useWorkspaceStore.getState().savePreset('doc:1', 'Drawing', saved)
  render(
    <WorkspaceDialog
      documentKey="doc:1"
      capture={() => saved}
      onApply={() => false}
      onClose={vi.fn()}
    />,
  )
  await act(async () => {
    fireEvent.click(screen.getByRole('button', { name: 'Apply Drawing' }))
  })
  expect(useWorkspaceStore.getState().getCurrent('doc:1')).toBeNull()
})

it('closes on Escape and exposes overwrite before replacing an existing name', () => {
  const saved = captureWorkspace(1, initialUiState())
  useWorkspaceStore.getState().savePreset('doc:1', 'Drawing', saved)
  const close = vi.fn()
  render(
    <WorkspaceDialog
      documentKey="doc:1"
      capture={() => saved}
      onApply={() => true}
      onClose={close}
    />,
  )
  const input = screen.getByRole('textbox', { name: 'Workspace name' })
  fireEvent.change(input, { target: { value: 'Drawing' } })
  expect(
    screen.getByRole('button', { name: 'Replace workspace' }),
  ).toBeEnabled()
  fireEvent.keyDown(input, { key: 'Escape' })
  expect(close).toHaveBeenCalledOnce()
})

it('follows the storage the kernel reports instead of reading it once', async () => {
  const saved = captureWorkspace(1, initialUiState())
  render(
    <WorkspaceDialog
      documentKey="path:/fonts/Family.fcfp"
      capture={() => saved}
      onApply={() => true}
      onClose={() => undefined}
    />,
  )

  // The kernel has not answered yet, so the dialog says what is true so far.
  expect(screen.getByText(/Saved on this computer only/)).toBeInTheDocument()

  act(() => {
    useWorkspaceStore
      .getState()
      .setStorage('path:/fonts/Family.fcfp', 'project')
  })

  expect(
    await screen.findByText(/Saved in this document's project/),
  ).toBeInTheDocument()
  expect(screen.queryByText(/Saved on this computer only/)).toBeNull()
})
