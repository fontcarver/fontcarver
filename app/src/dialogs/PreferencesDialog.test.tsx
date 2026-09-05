import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { setTransport, type Prefs, type SetPrefsParams } from '../api'
import {
  DEFAULT_PREFS,
  initialPrefsState,
  usePrefsStore,
} from '../stores/prefsStore'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { createMockTransport } from '../test/mockTransport'

import { PreferencesDialog } from './PreferencesDialog'

function setup(initial: Partial<Prefs> = {}) {
  let saved: Prefs = { ...DEFAULT_PREFS, ...initial }
  const transport = createMockTransport({
    'prefs.get': () => saved,
    'prefs.set': ({ prefs }) => {
      saved = prefs
      return prefs
    },
  })
  setTransport(transport)
  useUiStore.getState().setDialog({ kind: 'preferences' })
  render(<PreferencesDialog />)
  return transport
}

const lastPrefs = (transport: ReturnType<typeof setup>): Prefs | undefined =>
  (
    transport.calls.filter((entry) => entry.method === 'prefs.set').at(-1)
      ?.params as SetPrefsParams | undefined
  )?.prefs

beforeEach(() => {
  localStorage.clear()
  useUiStore.setState(initialUiState())
  usePrefsStore.setState(initialPrefsState())
})

afterEach(() => {
  setTransport(undefined)
})

describe('PreferencesDialog', () => {
  it('loads the kernel values on open', async () => {
    setup({ autosave_seconds: 15 })

    await waitFor(() =>
      expect(screen.getByLabelText('Autosave interval in seconds')).toHaveValue(
        15,
      ),
    )
  })

  it('applies a theme change immediately and persists it', async () => {
    const user = userEvent.setup()
    const transport = setup()
    await waitFor(() => expect(usePrefsStore.getState().loaded).toBe(true))

    await user.selectOptions(screen.getByLabelText('Theme'), 'dark')

    await waitFor(() => expect(lastPrefs(transport)?.theme).toBe('dark'))
    expect(useUiStore.getState().theme).toBe('dark')
  })

  it('switches the interface language through the shell', async () => {
    const user = userEvent.setup()
    const transport = setup()
    await waitFor(() => expect(usePrefsStore.getState().loaded).toBe(true))

    await user.selectOptions(screen.getByLabelText('Language'), 'zh-CN')

    await waitFor(() => expect(lastPrefs(transport)?.language).toBe('zh-CN'))
    expect(useUiStore.getState().language).toBe('zh-CN')
  })

  it('round trips the editor and snapping preferences', async () => {
    const user = userEvent.setup()
    const transport = setup()
    await waitFor(() => expect(usePrefsStore.getState().loaded).toBe(true))

    await user.click(screen.getByRole('tab', { name: 'Editor' }))
    const grid = screen.getByLabelText('Grid size')
    await user.clear(grid)
    await user.type(grid, '8')

    // Leaving the field commits it.
    await user.click(screen.getByRole('tab', { name: 'Snapping' }))
    await waitFor(() => expect(lastPrefs(transport)?.grid_size).toBe(8))
    await user.click(screen.getByRole('checkbox', { name: 'Guides' }))

    await waitFor(() => expect(lastPrefs(transport)?.snap.guides).toBe(false))
    expect(lastPrefs(transport)?.grid_size).toBe(8)

    // Re-reading the kernel returns exactly what the dialog wrote.
    usePrefsStore.setState(initialPrefsState())
    await usePrefsStore.getState().load()
    expect(usePrefsStore.getState().prefs.grid_size).toBe(8)
    expect(usePrefsStore.getState().prefs.snap.guides).toBe(false)
  })

  it('writes a number field once, when it is committed', async () => {
    const user = userEvent.setup()
    const transport = setup()
    await waitFor(() => expect(usePrefsStore.getState().loaded).toBe(true))

    await user.click(screen.getByRole('tab', { name: 'Editor' }))
    const grid = screen.getByLabelText('Grid size')
    await user.clear(grid)
    await user.type(grid, '16')

    // Clearing the field must not persist an intermediate zero.
    expect(
      transport.calls.filter((entry) => entry.method === 'prefs.set'),
    ).toHaveLength(0)

    await user.tab()
    await waitFor(() => expect(lastPrefs(transport)?.grid_size).toBe(16))
    expect(
      transport.calls.filter((entry) => entry.method === 'prefs.set'),
    ).toHaveLength(1)
  })

  it('commits a number field on Enter without closing the dialog', async () => {
    const user = userEvent.setup()
    const transport = setup()
    await waitFor(() => expect(usePrefsStore.getState().loaded).toBe(true))

    await user.click(screen.getByRole('tab', { name: 'Editor' }))
    const grid = screen.getByLabelText('Grid size')
    await user.clear(grid)
    await user.type(grid, '4{Enter}')

    await waitFor(() => expect(lastPrefs(transport)?.grid_size).toBe(4))
    expect(useUiStore.getState().dialog).toEqual({ kind: 'preferences' })
  })

  it('closes on Enter from a control that does not own the key', async () => {
    const user = userEvent.setup()
    setup()
    await waitFor(() => expect(usePrefsStore.getState().loaded).toBe(true))

    screen.getByLabelText('Theme').focus()
    await user.keyboard('{Enter}')

    expect(useUiStore.getState().dialog).toBeNull()
  })

  it('restores the defaults', async () => {
    const user = userEvent.setup()
    const transport = setup({ grid_size: 40, autosave_seconds: 5 })
    await waitFor(() => expect(usePrefsStore.getState().loaded).toBe(true))

    await user.click(screen.getByRole('button', { name: 'Reset to defaults' }))

    await waitFor(() => expect(lastPrefs(transport)).toEqual(DEFAULT_PREFS))
  })
})
