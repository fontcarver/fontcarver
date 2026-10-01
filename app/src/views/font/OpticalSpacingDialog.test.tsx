import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, expect, it, vi } from 'vitest'
import { setTransport, type OpticalBatchResult } from '../../api'
import { registerDocumentEditParticipant } from '../../editor/documentEditParticipants'
import { createMockTransport, noChange } from '../../test/mockTransport'
import { ApiCallError } from '../../transport'
import { OpticalSpacingDialog } from './OpticalSpacingDialog'
import { OpticalMetricsPreview } from './OpticalMetricsPreview'

const previewResult = (): OpticalBatchResult => ({
  generation: 12,
  glyph_count: 5,
  changed_count: 1,
  skipped: [
    { glyph: 'B', right: null, master: 'Regular', reason: 'locked' },
    { glyph: 'C', right: null, master: 'Regular', reason: 'locked' },
    { glyph: 'space', right: null, master: 'Regular', reason: 'empty_or_mark' },
    { glyph: 'Aacute', right: null, master: 'Regular', reason: 'metrics_key' },
  ],
  proposals: [
    {
      glyph: 'A',
      right: null,
      master: 'm1',
      master_name: 'Regular',
      before: [31, 47, 578],
      after: [53, 61, 614],
    },
  ],
  examples: [
    {
      glyph: 'A',
      right: null,
      master: 'Regular',
      before: [31, 47, 578],
      after: [53, 61, 614],
      outline: 'M31 0L531 0L531 700L31 700Z',
      after_outline: null,
      after_bounds: null,
      bounds: [31, 0, 531, 700],
    },
  ],
  change: null,
})

afterEach(() => setTransport(undefined))

it('requires a preview and shows actual sidebearings, widths and skipped reason counts', async () => {
  const mock = createMockTransport({ 'optical.batch': previewResult })
  setTransport(mock)
  render(
    <OpticalSpacingDialog
      doc={1}
      glyphs={[3, 7]}
      master="m1"
      onClose={vi.fn()}
    />,
  )
  const user = userEvent.setup()
  const apply = screen.getByRole('button', { name: 'Apply changes' })
  const dialog = screen.getByRole('dialog', {
    name: 'Optical spacing and kerning',
  })
  expect(dialog).toBeInTheDocument()
  expect(screen.getByRole('spinbutton', { name: 'Density' })).toHaveValue(1)
  expect(
    screen.getByText(/Larger values produce tighter spacing\./),
  ).toBeInTheDocument()
  expect(apply).toBeDisabled()
  expect(mock.calls).toHaveLength(0)

  await user.click(screen.getByRole('button', { name: 'Preview changes' }))
  const status = await screen.findByRole('status')
  expect(status).toHaveTextContent('1 changed layer across 5 selected glyphs')
  const table = screen.getByRole('table', { name: 'A · Regular' })
  for (const metric of ['Before', 'After', 'LSB', 'RSB', 'Width'])
    expect(table).toHaveTextContent(metric)
  for (const value of ['31', '47', '578', '53', '61', '614'])
    expect(table).toHaveTextContent(value)
  expect(dialog).toHaveTextContent('A · Regular')
  expect(dialog).toHaveTextContent('4 skipped layers')
  expect(dialog).toHaveTextContent('Locked glyph: 2')
  expect(dialog).toHaveTextContent('Empty outline or zero-width glyph: 1')
  expect(dialog).toHaveTextContent('Metrics key controls spacing: 1')
  expect(apply).toBeEnabled()
  expect(mock.calls).toEqual([
    {
      method: 'optical.batch',
      params: expect.objectContaining({
        doc: 1,
        glyphs: [3, 7],
        master: 'm1',
        operation: 'spacing',
        density: 1,
        replace_existing: false,
        dry_run: true,
        expected_generation: null,
      }),
    },
  ])
})

it('invalidates reviewed changes when density, masters, operation or kerning replacement change', async () => {
  const mock = createMockTransport({ 'optical.batch': previewResult })
  setTransport(mock)
  render(
    <OpticalSpacingDialog
      doc={1}
      glyphs={[3, 7]}
      master="m1"
      onClose={vi.fn()}
    />,
  )
  const user = userEvent.setup()
  const apply = screen.getByRole('button', { name: 'Apply changes' })
  const preview = async () => {
    await user.click(screen.getByRole('button', { name: 'Preview changes' }))
    expect(await screen.findByRole('status')).toBeInTheDocument()
    expect(apply).toBeEnabled()
  }
  const invalidated = () => {
    expect(apply).toBeDisabled()
    expect(screen.queryByRole('status')).not.toBeInTheDocument()
  }
  expect(
    screen.queryByRole('checkbox', { name: 'Replace existing kerning' }),
  ).not.toBeInTheDocument()
  await preview()
  fireEvent.change(screen.getByRole('spinbutton', { name: 'Density' }), {
    target: { value: '2' },
  })
  invalidated()
  await preview()
  await user.click(screen.getByRole('checkbox', { name: 'All masters' }))
  invalidated()
  await preview()
  await user.selectOptions(
    screen.getByRole('combobox', { name: 'Operation' }),
    'kerning',
  )
  invalidated()
  const replace = screen.getByRole('checkbox', {
    name: 'Replace existing kerning',
  })
  expect(replace).not.toBeChecked()
  await preview()
  await user.click(replace)
  invalidated()
  await preview()
  expect(mock.calls.at(-1)).toMatchObject({
    method: 'optical.batch',
    params: {
      master: null,
      operation: 'kerning',
      density: 2,
      replace_existing: true,
      dry_run: true,
    },
  })
})

it('applies the reviewed document, selection, master and generation after props change', async () => {
  const close = vi.fn()
  const mock = createMockTransport({
    'optical.batch': (params) => ({
      ...previewResult(),
      change: params.dry_run ? null : noChange(),
    }),
  })
  setTransport(mock)
  const { rerender } = render(
    <OpticalSpacingDialog
      doc={1}
      glyphs={[3, 7]}
      master="m1"
      onClose={close}
    />,
  )
  const user = userEvent.setup()
  await user.click(screen.getByRole('button', { name: 'Preview changes' }))
  await screen.findByRole('status')
  rerender(
    <OpticalSpacingDialog doc={2} glyphs={[9]} master="m2" onClose={close} />,
  )
  await user.click(screen.getByRole('button', { name: 'Apply changes' }))
  await waitFor(() => expect(close).toHaveBeenCalledOnce())
  expect(mock.calls).toHaveLength(2)
  expect(mock.calls[1]).toMatchObject({
    method: 'optical.batch',
    params: {
      doc: 1,
      glyphs: [3, 7],
      master: 'm1',
      expected_generation: 12,
      dry_run: false,
    },
  })
})

it('clears a stale preview and reports the rejected apply without closing', async () => {
  const close = vi.fn()
  const mock = createMockTransport({
    'optical.batch': (params) => {
      if (!params.dry_run)
        throw new ApiCallError('invalid_params', 'Optical preview is stale')
      return previewResult()
    },
  })
  setTransport(mock)
  render(
    <OpticalSpacingDialog
      doc={1}
      glyphs={[3, 7]}
      master="m1"
      onClose={close}
    />,
  )
  const user = userEvent.setup()
  await user.click(screen.getByRole('button', { name: 'Preview changes' }))
  await screen.findByRole('status')
  const apply = screen.getByRole('button', { name: 'Apply changes' })
  await user.click(apply)
  await waitFor(() => expect(apply).toBeDisabled())
  expect(screen.queryByRole('status')).not.toBeInTheDocument()
  expect(close).not.toHaveBeenCalled()
  const alert = await screen.findByRole('alert')
  expect(alert).toHaveTextContent('Optical preview is stale')
  await user.click(apply)
  expect(mock.calls).toHaveLength(2)
  await user.click(screen.getByRole('button', { name: 'Preview changes' }))
  expect(await screen.findByRole('status')).toBeInTheDocument()
  expect(apply).toBeEnabled()
})

it('keeps apply disabled when the preview contains no changes', async () => {
  const mock = createMockTransport({
    'optical.batch': () => ({
      ...previewResult(),
      changed_count: 0,
      proposals: [],
      examples: [],
    }),
  })
  setTransport(mock)
  render(
    <OpticalSpacingDialog doc={1} glyphs={[3]} master="m1" onClose={vi.fn()} />,
  )
  await userEvent
    .setup()
    .click(screen.getByRole('button', { name: 'Preview changes' }))
  expect(await screen.findByRole('status')).toHaveTextContent(
    '0 changed layers',
  )
  expect(
    screen.getByText('No changes are needed with these settings.'),
  ).toBeInTheDocument()
  expect(screen.getByRole('button', { name: 'Apply changes' })).toBeDisabled()
  expect(mock.calls).toHaveLength(1)
})

it('reports a failed draft flush and makes no optical request', async () => {
  const flush = vi.fn(async () => {
    throw new Error('Pending metrics could not be saved')
  })
  const unregister = registerDocumentEditParticipant(1, { flush })
  const mock = createMockTransport({ 'optical.batch': previewResult })
  setTransport(mock)
  try {
    render(
      <OpticalSpacingDialog
        doc={1}
        glyphs={[3]}
        master="m1"
        onClose={vi.fn()}
      />,
    )
    await userEvent
      .setup()
      .click(screen.getByRole('button', { name: 'Preview changes' }))
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Pending metrics could not be saved',
    )
    expect(flush).toHaveBeenCalledOnce()
    expect(mock.calls).toHaveLength(0)
    expect(screen.getByRole('button', { name: 'Apply changes' })).toBeDisabled()
  } finally {
    unregister()
  }
})

it.each(['', 'NaN', 'Infinity', '0.24', '4.01'])(
  'rejects density %j before calling the API',
  async (density) => {
    const mock = createMockTransport({ 'optical.batch': previewResult })
    setTransport(mock)
    render(
      <OpticalSpacingDialog
        doc={1}
        glyphs={[3]}
        master="m1"
        onClose={vi.fn()}
      />,
    )
    fireEvent.change(screen.getByRole('spinbutton', { name: 'Density' }), {
      target: { value: density },
    })
    await userEvent
      .setup()
      .click(screen.getByRole('button', { name: 'Preview changes' }))
    expect(mock.calls).toHaveLength(0)
    expect(screen.getByRole('button', { name: 'Apply changes' })).toBeDisabled()
    expect(screen.queryByRole('status')).not.toBeInTheDocument()
  },
)

it('renders the backend resolved composite outline without a second translation', () => {
  const example = {
    ...previewResult().examples[0],
    after_outline: 'M11 0L541 0L541 700L11 700Z',
    after_bounds: [11, 0, 541, 700] as [number, number, number, number],
  }
  const { container } = render(
    <OpticalMetricsPreview examples={[example]} operation="spacing" />,
  )
  const paths = container.querySelectorAll('path')
  expect(paths[0]).toHaveAttribute('d', example.outline)
  expect(paths[1]).toHaveAttribute('d', example.after_outline)
  expect(paths[1]).toHaveAttribute('transform', 'translate(0 0)')
})

const pairProposals = (count: number) =>
  Array.from({ length: count }, (_, index) => ({
    glyph: 'A',
    right: `g${index}`,
    master: 'm1',
    master_name: 'Regular',
    before: [0, 0, 0] as [number, number, number],
    after: [-index - 1, 0, 0] as [number, number, number],
  }))

it('applies only the accepted rows and refuses to apply with none', async () => {
  const result: OpticalBatchResult = {
    ...previewResult(),
    changed_count: 3,
    proposals: pairProposals(3),
  }
  const mock = createMockTransport({
    'optical.batch': ({ dry_run }) =>
      dry_run ? result : { ...result, change: noChange([1]) },
  })
  setTransport(mock)
  render(
    <OpticalSpacingDialog doc={1} glyphs={[3]} master="m1" onClose={vi.fn()} />,
  )
  const user = userEvent.setup()
  await user.click(screen.getByRole('button', { name: 'Preview changes' }))
  await screen.findByRole('table', { name: /Every proposed change/ })
  expect(screen.getByText('3 of 3 accepted')).toBeInTheDocument()
  await user.click(screen.getByRole('checkbox', { name: 'Accept A / g1' }))
  expect(screen.getByText('2 of 3 accepted')).toBeInTheDocument()
  await user.click(screen.getByRole('button', { name: 'Apply changes' }))
  await waitFor(() => expect(mock.calls).toHaveLength(2))
  expect(mock.calls[1]?.params).toMatchObject({
    dry_run: false,
    expected_generation: 12,
    accepted: [
      { glyph: 'A', right: 'g0', master: 'm1' },
      { glyph: 'A', right: 'g2', master: 'm1' },
    ],
  })
})

it('rejecting every row disables apply and says so', async () => {
  const result: OpticalBatchResult = {
    ...previewResult(),
    changed_count: 2,
    proposals: pairProposals(2),
  }
  const mock = createMockTransport({ 'optical.batch': () => result })
  setTransport(mock)
  render(
    <OpticalSpacingDialog doc={1} glyphs={[3]} master="m1" onClose={vi.fn()} />,
  )
  const user = userEvent.setup()
  await user.click(screen.getByRole('button', { name: 'Preview changes' }))
  await user.click(screen.getByRole('button', { name: 'Reject all' }))
  expect(screen.getByText('0 of 2 accepted')).toBeInTheDocument()
  expect(
    screen.getByText('Accept at least one row before applying.'),
  ).toBeInTheDocument()
  expect(screen.getByRole('button', { name: 'Apply changes' })).toBeDisabled()
  await user.click(screen.getByRole('button', { name: 'Accept all' }))
  expect(screen.getByRole('button', { name: 'Apply changes' })).toBeEnabled()
  expect(mock.calls).toHaveLength(1)
})

it('windows a large proposal and keeps select-all over every row', async () => {
  const result: OpticalBatchResult = {
    ...previewResult(),
    changed_count: 250,
    proposals: pairProposals(250),
  }
  const mock = createMockTransport({
    'optical.batch': ({ dry_run }) =>
      dry_run ? result : { ...result, change: noChange([1]) },
  })
  setTransport(mock)
  render(
    <OpticalSpacingDialog doc={1} glyphs={[3]} master="m1" onClose={vi.fn()} />,
  )
  const user = userEvent.setup()
  await user.click(screen.getByRole('button', { name: 'Preview changes' }))
  expect(await screen.findByText('Showing 200 of 250 rows.')).toBeVisible()
  expect(
    screen.getAllByRole('checkbox', { name: /^Accept A \// }),
  ).toHaveLength(200)
  await user.click(screen.getByRole('button', { name: 'Show more rows' }))
  expect(
    screen.getAllByRole('checkbox', { name: /^Accept A \// }),
  ).toHaveLength(250)
  await user.click(screen.getByRole('button', { name: 'Apply changes' }))
  await waitFor(() => expect(mock.calls).toHaveLength(2))
  const accepted = (mock.calls[1]?.params as { accepted: unknown[] }).accepted
  expect(accepted).toHaveLength(250)
})

it('a new preview clears earlier rejections', async () => {
  const result: OpticalBatchResult = {
    ...previewResult(),
    changed_count: 2,
    proposals: pairProposals(2),
  }
  const mock = createMockTransport({ 'optical.batch': () => result })
  setTransport(mock)
  render(
    <OpticalSpacingDialog doc={1} glyphs={[3]} master="m1" onClose={vi.fn()} />,
  )
  const user = userEvent.setup()
  await user.click(screen.getByRole('button', { name: 'Preview changes' }))
  await user.click(screen.getByRole('button', { name: 'Reject all' }))
  expect(screen.getByText('0 of 2 accepted')).toBeInTheDocument()
  fireEvent.change(screen.getByRole('spinbutton', { name: 'Density' }), {
    target: { value: '2' },
  })
  await user.click(screen.getByRole('button', { name: 'Preview changes' }))
  expect(await screen.findByText('2 of 2 accepted')).toBeVisible()
})
