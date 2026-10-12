import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, expect, it, vi } from 'vitest'

import { setTransport } from '../api'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { createMockTransport } from '../test/mockTransport'
import { MacroConsole } from './MacroConsole'
import { initialAutomationState, useAutomationStore } from './automationStore'

vi.mock('../plugins/PluginManager', () => ({ PluginManager: () => null }))

beforeEach(() => {
  useUiStore.setState(initialUiState())
  useAutomationStore.setState({ ...initialAutomationState(), open: true })
})

it('reloads and selects scripts without running them, then displays explicit Python errors', async () => {
  const user = userEvent.setup()
  const transport = createMockTransport({
    'automation.scripts_reload': () => ({
      folders: [{ kind: 'preferences', path: '/config/scripts', exists: true }],
      scripts: [
        {
          name: 'example.py',
          folder: 'preferences',
          language: 'python',
          bytes: 10,
        },
      ],
      warnings: [],
      fingerprint: 'aaa',
    }),
    'automation.settings_get': () => ({ scripts_directory: null }),
    'automation.script_read': () => ({
      name: 'example.py',
      folder: 'preferences',
      language: 'python',
      source: 'print("hello")',
    }),
    'automation.python_run': () => ({
      output: ['hello'],
      error: 'Script failed',
      elapsed_ms: 12,
      rpc_calls: 0,
      undo_label: null,
    }),
  })
  setTransport(transport)
  render(<MacroConsole />)
  await user.click(screen.getByRole('button', { name: 'Reload scripts' }))
  expect(await screen.findByText('/config/scripts')).toBeVisible()
  await user.selectOptions(
    screen.getByLabelText('Local script'),
    'preferences/example.py',
  )
  await waitFor(() =>
    expect(screen.getByLabelText('Script source')).toHaveValue(
      'print("hello")',
    ),
  )
  expect(
    transport.calls
      .map(({ method }) => method)
      .filter((method) => method !== 'automation.settings_get'),
  ).toEqual(['automation.scripts_reload', 'automation.script_read'])
  await user.click(screen.getByRole('button', { name: 'Run script' }))
  expect(await screen.findByText('Script failed')).toBeVisible()
  expect(screen.getByRole('log', { name: 'Output' })).toHaveTextContent('hello')
  expect(screen.getByRole('status')).toHaveTextContent('Failed')
}, 15000)

it('explains the Python deadline and prevents closing while it is running', async () => {
  const user = userEvent.setup()
  useAutomationStore.setState({
    language: 'python',
    running: true,
    status: 'running',
  })
  render(<MacroConsole />)
  expect(screen.getByText(/manual cancellation is unavailable/)).toBeVisible()
  expect(screen.getByRole('button', { name: 'Close' })).toBeDisabled()
  expect(
    screen.queryByRole('button', { name: 'Stop script' }),
  ).not.toBeInTheDocument()
  await user.keyboard('{Escape}')
  expect(screen.getByRole('dialog')).toBeVisible()
})
