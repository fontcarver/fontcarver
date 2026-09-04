import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { ApiCallError } from '../transport'
import {
  initialUiState,
  reportError,
  useUiStore,
  type Toast as ToastData,
} from '../stores/uiStore'

import { Toast } from './Toast'
import { ToastStack } from './ToastStack'

function toast(overrides: Partial<ToastData> = {}): ToastData {
  return { id: 't1', kind: 'error', message: 'Something failed', ...overrides }
}

beforeEach(() => {
  useUiStore.setState(initialUiState())
})

afterEach(() => {
  useUiStore.setState(initialUiState())
})

describe('Toast', () => {
  it('hides the kernel detail behind a Details disclosure', async () => {
    render(
      <Toast
        toast={toast({ details: 'no glyph with handle 7' })}
        onDismiss={vi.fn()}
      />,
    )

    const details = screen.getByText('Details')
    expect(screen.getByText('no glyph with handle 7')).not.toBeVisible()

    await userEvent.click(details)
    expect(screen.getByText('no glyph with handle 7')).toBeVisible()
  })

  it('shows no disclosure when there is nothing beyond the message', () => {
    render(<Toast toast={toast()} onDismiss={vi.fn()} />)

    expect(screen.queryByText('Details')).not.toBeInTheDocument()
  })
})

describe('reportError', () => {
  it('pushes a translated toast with the kernel wording as detail', () => {
    reportError(new ApiCallError('parse', 'unsupported table version'))

    const [pushed] = useUiStore.getState().toasts
    expect(pushed?.kind).toBe('error')
    expect(pushed?.message).not.toBe('unsupported table version')
    expect(pushed?.details).toBe('unsupported table version')
  })

  it('renders the translated summary in the stack', () => {
    reportError(new ApiCallError('io', 'permission denied'))
    render(<ToastStack />)

    expect(screen.getByRole('alert')).toHaveTextContent('Details')
    expect(screen.queryByText('permission denied')).not.toBeVisible()
  })

  it('keeps an untranslatable failure readable', () => {
    reportError(new Error('the network went away'))

    const [pushed] = useUiStore.getState().toasts
    expect(pushed?.message).toBe('the network went away')
    expect(pushed?.details).toBeUndefined()
  })
})
