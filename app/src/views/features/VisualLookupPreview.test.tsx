import { act, fireEvent, render, screen, within } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import { setTransport, type ShapedRun } from '../../api'
import { createMockTransport } from '../../test/mockTransport'
import { VisualLookupPreview } from './VisualLookupPreview'

const props = {
  doc: 1,
  master: 'm1',
  generation: 1,
  tag: 'liga',
  hasDraft: false,
}
const run = (name = 'f_i', advance = 850): ShapedRun => ({
  glyphs: [
    {
      gid: 7,
      cluster: 0,
      x_advance: advance,
      y_advance: 0,
      x_offset: 12,
      y_offset: -5,
    },
  ],
  names: [name],
  warnings: [],
  stale: false,
  units_per_em: 1000,
  size: null,
})
const preview = () =>
  fireEvent.click(screen.getByRole('button', { name: 'Preview shaping' }))
afterEach(() => setTransport(undefined))

describe('VisualLookupPreview', () => {
  it('shapes only on request and compares real glyph names and font-unit metrics', async () => {
    const transport = createMockTransport({
      'text.shape': ({ features }) => (features[0][1] ? run() : run('f', 1000)),
    })
    setTransport(transport)
    render(<VisualLookupPreview {...props} hasDraft />)
    expect(transport.calls).toEqual([])
    expect(screen.getByText(/uses the applied source/)).toBeVisible()
    fireEvent.change(screen.getByRole('textbox', { name: 'Preview text' }), {
      target: { value: 'fi' },
    })
    preview()
    expect(await screen.findByText('f_i')).toBeVisible()
    const enabled = screen.getByRole('table', { name: 'liga on' })
    expect(within(enabled).getByText('850')).toBeVisible()
    expect(within(enabled).getByText('12')).toBeVisible()
    expect(within(enabled).getByText('-5')).toBeVisible()
    expect(
      screen.getByText(
        'Advance difference (on − off): x −150, y 0 font units.',
      ),
    ).toBeVisible()
    expect(transport.calls).toEqual(
      [false, true].map((enabled) => ({
        method: 'text.shape',
        params: {
          doc: 1,
          master: 'm1',
          text: 'fi',
          features: [['liga', enabled]],
          script: null,
          language: null,
          direction: null,
          size: null,
        },
      })),
    )
  })

  it.each([{ doc: 2 }, { master: 'm2' }, { generation: 2 }, { tag: 'kern' }])(
    'discards pending results after context changes: %j',
    async (changed) => {
      const pending: ((value: ShapedRun) => void)[] = []
      const transport = createMockTransport({
        'text.shape': () =>
          new Promise<ShapedRun>((resolve) => pending.push(resolve)),
      })
      setTransport(transport)
      const view = render(<VisualLookupPreview {...props} />)
      fireEvent.change(screen.getByRole('textbox', { name: 'Preview text' }), {
        target: { value: 'fi' },
      })
      preview()
      view.rerender(<VisualLookupPreview {...props} {...changed} />)
      await act(async () =>
        pending.forEach((resolve) => resolve(run('obsolete'))),
      )
      expect(screen.queryByText('obsolete')).not.toBeInTheDocument()
      expect(transport.calls).toHaveLength(2)
    },
  )

  it('invalidates results when text changes and keeps the newest preview', async () => {
    const pending: ((value: ShapedRun) => void)[] = []
    setTransport(
      createMockTransport({
        'text.shape': ({ text }) =>
          text === 'fi'
            ? new Promise<ShapedRun>((resolve) => pending.push(resolve))
            : run('current'),
      }),
    )
    render(<VisualLookupPreview {...props} />)
    const input = screen.getByRole('textbox', { name: 'Preview text' })
    fireEvent.change(input, { target: { value: 'fi' } })
    preview()
    fireEvent.change(input, { target: { value: 'AV' } })
    preview()
    await screen.findAllByText('current')
    await act(async () =>
      pending.forEach((resolve) => resolve(run('obsolete'))),
    )
    expect(screen.queryByText('obsolete')).not.toBeInTheDocument()
    expect(screen.getAllByText('current')).toHaveLength(2)
  })

  it('rejects invalid tags and empty input and caps preview text at 128 characters', () => {
    const transport = createMockTransport()
    setTransport(transport)
    const view = render(<VisualLookupPreview {...props} tag="bad" />)
    expect(
      screen.getByRole('button', { name: 'Preview shaping' }),
    ).toBeDisabled()
    view.rerender(<VisualLookupPreview {...props} />)
    expect(
      screen.getByRole('button', { name: 'Preview shaping' }),
    ).toBeDisabled()
    expect(
      screen.getByRole('textbox', { name: 'Preview text' }),
    ).toHaveAttribute('maxlength', '128')
    expect(transport.calls).toEqual([])
  })

  it('reports outdated shaping fonts instead of presenting a current comparison', async () => {
    setTransport(
      createMockTransport({
        'text.shape': () => ({ ...run('outdated'), stale: true }),
      }),
    )
    render(<VisualLookupPreview {...props} />)
    fireEvent.change(screen.getByRole('textbox', { name: 'Preview text' }), {
      target: { value: 'fi' },
    })
    preview()
    expect(await screen.findByText(/font is still rebuilding/)).toBeVisible()
    expect(screen.queryByRole('table')).not.toBeInTheDocument()
  })
})
