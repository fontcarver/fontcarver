import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import {
  setTransport,
  type MetricsKeys,
  type ResolvedMetrics,
} from '../../../api'
import { createMockTransport } from '../../../test/mockTransport'

import { useResolvedMetrics } from './useResolvedMetrics'

function Probe({
  keys,
  fallback,
  apply,
}: {
  keys: MetricsKeys
  fallback: ResolvedMetrics
  apply: (value: ResolvedMetrics) => void
}) {
  const resolution = useResolvedMetrics(1, 42, 'm1', keys, fallback)
  return (
    <>
      <output>{`${resolution.status}:${resolution.value.lsb}`}</output>
      <button
        type="button"
        disabled={resolution.status !== 'ready'}
        onClick={() => apply(resolution.value)}
      >
        Apply
      </button>
    </>
  )
}

afterEach(() => setTransport(undefined))

describe('useResolvedMetrics', () => {
  it('never exposes a previous formula result while a new identity is pending', async () => {
    const pending: ((value: ResolvedMetrics) => void)[] = []
    setTransport(
      createMockTransport({
        'doc.metrics_resolve': () =>
          new Promise<ResolvedMetrics>((resolve) => pending.push(resolve)),
      }),
    )
    const apply = vi.fn()
    const firstKeys: MetricsKeys = {
      left: '=H',
      right: null,
      width: null,
      vertical_origin: null,
      vertical_width: null,
    }
    const fallback = (lsb: number): ResolvedMetrics => ({
      lsb,
      rsb: 60,
      width: 900,
      source: 'Explicit',
    })
    const view = render(
      <Probe keys={firstKeys} fallback={fallback(10)} apply={apply} />,
    )
    await waitFor(() => expect(pending).toHaveLength(1))
    pending[0]?.({ lsb: 100, rsb: 40, width: 660, source: 'Key' })
    expect(await screen.findByText('ready:100')).toBeInTheDocument()

    view.rerender(
      <Probe
        keys={{ ...firstKeys, left: '=n' }}
        fallback={fallback(50)}
        apply={apply}
      />,
    )
    expect(screen.getByText('pending:50')).toBeInTheDocument()
    const button = screen.getByRole('button', { name: 'Apply' })
    expect(button).toBeDisabled()
    await userEvent.click(button)
    expect(apply).not.toHaveBeenCalled()
  })
})
