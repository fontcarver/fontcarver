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
  doc = 1,
  keys,
  fallback,
  apply,
}: {
  doc?: number
  keys: MetricsKeys
  fallback: ResolvedMetrics
  apply: (value: ResolvedMetrics) => void
}) {
  const resolution = useResolvedMetrics(doc, 42, 'm1', keys, fallback)
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

  it('makes a resolved value pending when a referenced glyph changes', async () => {
    const pending: ((value: ResolvedMetrics) => void)[] = []
    const transport = createMockTransport({
      'doc.metrics_resolve': () =>
        new Promise<ResolvedMetrics>((resolve) => pending.push(resolve)),
    })
    setTransport(transport)
    const apply = vi.fn()
    const keys: MetricsKeys = {
      left: '=H',
      right: null,
      width: null,
      vertical_origin: null,
      vertical_width: null,
    }
    const fallback: ResolvedMetrics = {
      lsb: 10,
      rsb: 60,
      width: 900,
      source: 'Explicit',
    }
    render(<Probe keys={keys} fallback={fallback} apply={apply} />)
    await waitFor(() => expect(pending).toHaveLength(1))
    pending[0]?.({ lsb: 100, rsb: 40, width: 660, source: 'Key' })
    expect(await screen.findByText('ready:100')).toBeInTheDocument()

    transport.emit({
      doc: 1,
      generation: 7,
      dirty: true,
      undo_label: 'Set metrics',
      redo_label: null,
      change: {
        glyphs: [99],
        font_info: false,
        masters: false,
        glyph_order: false,
        metrics: true,
        layers: false,
        guides: false,
      },
    })

    expect(await screen.findByText('pending:10')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Apply' })).toBeDisabled()
    await waitFor(() => expect(pending).toHaveLength(2))
    transport.emit({
      doc: 1,
      generation: 8,
      dirty: true,
      undo_label: 'Move outline',
      redo_label: null,
      change: {
        glyphs: [99],
        font_info: false,
        masters: false,
        glyph_order: false,
        metrics: true,
        layers: false,
        guides: false,
      },
    })
    await waitFor(() => expect(pending).toHaveLength(3))
    pending[1]?.({ lsb: 110, rsb: 40, width: 670, source: 'Key' })
    expect(screen.getByText('pending:10')).toBeInTheDocument()
    pending[2]?.({ lsb: 120, rsb: 40, width: 680, source: 'Key' })
    expect(await screen.findByText('ready:120')).toBeInTheDocument()
  })

  it('does not carry a high document generation into another document', async () => {
    const pending: ((value: ResolvedMetrics) => void)[] = []
    const transport = createMockTransport({
      'doc.metrics_resolve': () =>
        new Promise<ResolvedMetrics>((resolve) => pending.push(resolve)),
    })
    setTransport(transport)
    const keys: MetricsKeys = {
      left: '=H',
      right: null,
      width: null,
      vertical_origin: null,
      vertical_width: null,
    }
    const fallback: ResolvedMetrics = {
      lsb: 10,
      rsb: 60,
      width: 900,
      source: 'Explicit',
    }
    const view = render(
      <Probe keys={keys} fallback={fallback} apply={() => undefined} />,
    )
    await waitFor(() => expect(pending).toHaveLength(1))
    pending[0]?.({ lsb: 100, rsb: 40, width: 660, source: 'Key' })
    expect(await screen.findByText('ready:100')).toBeInTheDocument()

    transport.emit({
      doc: 1,
      generation: 100,
      dirty: true,
      undo_label: 'Set metrics',
      redo_label: null,
      change: {
        glyphs: [99],
        font_info: false,
        masters: false,
        glyph_order: false,
        metrics: true,
        layers: false,
        guides: false,
      },
    })
    await waitFor(() => expect(pending).toHaveLength(2))
    pending[1]?.({ lsb: 110, rsb: 40, width: 670, source: 'Key' })
    expect(await screen.findByText('ready:110')).toBeInTheDocument()

    view.rerender(
      <Probe doc={2} keys={keys} fallback={fallback} apply={() => undefined} />,
    )
    await waitFor(() => expect(pending).toHaveLength(3))
    pending[2]?.({ lsb: 120, rsb: 40, width: 680, source: 'Key' })
    expect(await screen.findByText('ready:120')).toBeInTheDocument()

    transport.emit({
      doc: 2,
      generation: 2,
      dirty: true,
      undo_label: 'Set metrics',
      redo_label: null,
      change: {
        glyphs: [99],
        font_info: false,
        masters: false,
        glyph_order: false,
        metrics: true,
        layers: false,
        guides: false,
      },
    })
    expect(await screen.findByText('pending:10')).toBeInTheDocument()
    await waitFor(() => expect(pending).toHaveLength(4))
  })
})
