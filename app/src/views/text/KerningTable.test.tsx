import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it } from 'vitest'
import { setTransport, type Command } from '../../api'
import { createMockTransport, noChange } from '../../test/mockTransport'
import { KerningTable } from './KerningTable'

const items = [
  {
    left: { group: 'public.kern1.A' },
    right: { group: 'public.kern2.V' },
    value: -80,
    diagnostic: null,
    exception: false,
  },
  {
    left: { glyph: 'A' },
    right: { glyph: 'V' },
    value: 0,
    diagnostic: null,
    exception: true,
  },
]
let commands: Command[]
beforeEach(() => {
  commands = []
  setTransport(
    createMockTransport({
      'kerning.pairs': () => ({
        status: { generation: 4, editable: true, reason: null },
        total: items.length,
        items,
      }),
      'kerning.pairs_vertical': () => ({
        status: { generation: 4, editable: true, reason: null },
        total: items.length,
        items,
      }),
      'kerning.groups': () => ({
        status: { generation: 4, editable: true, reason: null },
        total: 1,
        items: [
          {
            name: 'public.kern1.A',
            member_count: 2,
            pair_count: 1,
            sample_members: ['A'],
            diagnostic: null,
          },
        ],
      }),
      'doc.apply': ({ command }) => {
        commands.push(command)
        return noChange()
      },
    }),
  )
})
function openTable() {
  fireEvent.click(screen.getByText('Kerning table'))
}

describe('kerning table', () => {
  it('loads only after opening and filters stored group records', async () => {
    render(
      <KerningTable doc={1} master="regular" generation={4} vertical={false} />,
    )
    expect(screen.queryByRole('table')).not.toBeInTheDocument()
    openTable()
    await screen.findByRole('table')
    fireEvent.change(screen.getByLabelText('Filter pairs'), {
      target: { value: 'kern1' },
    })
    expect(screen.getByText('public.kern1.A')).toBeInTheDocument()
    expect(
      screen.queryByRole('button', { name: 'Remove A / V' }),
    ).not.toBeInTheDocument()
  })

  it('copies explicit zero and classes then pastes into the newly selected master', async () => {
    const view = render(
      <KerningTable doc={1} master="regular" generation={4} vertical={false} />,
    )
    openTable()
    fireEvent.click(await screen.findByRole('button', { name: 'Copy table' }))
    view.rerender(
      <KerningTable doc={1} master="bold" generation={4} vertical={false} />,
    )
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Paste table' })).toBeEnabled(),
    )
    fireEvent.click(screen.getByRole('button', { name: 'Paste table' }))
    await waitFor(() => expect(commands).toHaveLength(1))
    expect(commands[0]).toEqual({
      KerningMaintenance: {
        master: 'bold',
        vertical: false,
        expected_generation: 4,
        operation: {
          Paste: {
            records: items.map(({ left, right, value }) => ({
              left,
              right,
              value,
            })),
            replace: false,
          },
        },
      },
    })
  })

  it('passes the current direction and generation to maintenance', async () => {
    render(<KerningTable doc={1} master="regular" generation={4} vertical />)
    openTable()
    fireEvent.click(await screen.findByRole('button', { name: 'Compress' }))
    await waitFor(() =>
      expect(commands).toEqual([
        {
          KerningMaintenance: {
            master: 'regular',
            vertical: true,
            expected_generation: 4,
            operation: 'Compress',
          },
        },
      ]),
    )
  })

  it('cleans up obsolete records, not only invalid ones', async () => {
    render(
      <KerningTable doc={1} master="regular" generation={4} vertical={false} />,
    )
    openTable()
    fireEvent.click(await screen.findByRole('button', { name: 'Clean Up' }))
    await waitFor(() =>
      expect(commands).toEqual([
        {
          KerningMaintenance: {
            master: 'regular',
            vertical: false,
            expected_generation: 4,
            operation: 'CleanupObsolete',
          },
        },
      ]),
    )
  })

  it('asks for the rename scope and states what each one changes', async () => {
    render(
      <KerningTable doc={1} master="regular" generation={4} vertical={false} />,
    )
    openTable()
    fireEvent.click(
      await screen.findByRole('button', { name: 'Rename public.kern1.A' }),
    )
    expect(screen.getByRole('dialog')).toHaveTextContent('every master')
    fireEvent.change(screen.getByLabelText('New group name'), {
      target: { value: 'A.new' },
    })
    expect(commands).toHaveLength(0)
    fireEvent.click(screen.getByRole('button', { name: 'Rename' }))
    await waitFor(() =>
      expect(commands).toEqual([
        {
          RenameKerningGroup: {
            side: 'side1',
            from: 'public.kern1.A',
            to: 'public.kern1.A.new',
          },
        },
      ]),
    )
  })

  it('retargets only the chosen record when that scope is selected', async () => {
    render(
      <KerningTable doc={1} master="regular" generation={4} vertical={false} />,
    )
    openTable()
    fireEvent.click(
      await screen.findByRole('button', { name: 'Rename public.kern1.A' }),
    )
    fireEvent.click(
      screen.getByRole('radio', { name: 'Retarget this record only' }),
    )
    expect(screen.getByRole('dialog')).toHaveTextContent(
      'Only this record moves',
    )
    fireEvent.change(screen.getByLabelText('New group name'), {
      target: { value: 'A.alt' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Rename' }))
    await waitFor(() => expect(commands).toHaveLength(1))
    expect(commands[0]).toEqual({
      Batch: [
        {
          SetHorizontalKerning: {
            master: 'regular',
            left: { group: 'public.kern1.A' },
            right: { group: 'public.kern2.V' },
            value: null,
          },
        },
        {
          SetHorizontalKerning: {
            master: 'regular',
            left: { group: 'public.kern1.A.alt' },
            right: { group: 'public.kern2.V' },
            value: -80,
          },
        },
      ],
    })
  })

  it('offers existing groups and refuses to retarget an unreadable record', async () => {
    setTransport(
      createMockTransport({
        'kerning.pairs': () => ({
          status: { generation: 4, editable: true, reason: null },
          total: 1,
          items: [
            {
              left: { group: 'public.kern1.A' },
              right: { glyph: 'V' },
              // An imported non-finite value the table cannot read.
              value: null,
              diagnostic: 'kerning must be finite',
              exception: false,
            },
          ],
        }),
        'kerning.groups': () => ({
          status: { generation: 4, editable: true, reason: null },
          total: 2,
          items: [
            {
              name: 'public.kern1.A',
              member_count: 2,
              pair_count: 1,
              sample_members: ['A'],
              diagnostic: null,
            },
            {
              name: 'public.kern1.Diagonal',
              member_count: 1,
              pair_count: 0,
              sample_members: ['V'],
              diagnostic: null,
            },
          ],
        }),
        'doc.apply': () => noChange(),
      }),
    )
    render(
      <KerningTable doc={1} master="regular" generation={4} vertical={false} />,
    )
    openTable()
    fireEvent.click(
      await screen.findByRole('button', { name: 'Rename public.kern1.A' }),
    )
    const retarget = screen.getByRole('radio', {
      name: 'Retarget this record only',
    })
    expect(retarget).toBeDisabled()
    expect(screen.getByRole('dialog')).toHaveTextContent('cannot be retargeted')
    // The existing groups of this side are offered as completions.
    await waitFor(() =>
      expect(
        screen.getByRole('dialog').querySelectorAll('datalist option'),
      ).toHaveLength(2),
    )
  })

  it('marks exceptions, class rules and plain pairs, and filters to exceptions', async () => {
    render(
      <KerningTable doc={1} master="regular" generation={4} vertical={false} />,
    )
    openTable()
    await screen.findByRole('table')
    expect(screen.getByText('Group')).toBeInTheDocument()
    expect(screen.getByText('Exception')).toBeInTheDocument()
    fireEvent.change(screen.getByLabelText('Record kind'), {
      target: { value: 'exceptions' },
    })
    expect(screen.queryByText('public.kern1.A')).not.toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: 'Remove A / V' }),
    ).toBeInTheDocument()
  })

  it('inspects another master without moving the proof and pastes there', async () => {
    const masters = [
      { id: 'regular', name: 'Regular' },
      { id: 'bold', name: 'Bold' },
    ]
    render(
      <KerningTable
        doc={1}
        master="regular"
        masters={masters}
        generation={4}
        vertical={false}
      />,
    )
    openTable()
    fireEvent.click(await screen.findByRole('button', { name: 'Copy table' }))
    fireEvent.change(screen.getByLabelText('Table master'), {
      target: { value: 'bold' },
    })
    fireEvent.change(screen.getByLabelText('On conflict'), {
      target: { value: 'keep' },
    })
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Paste table' })).toBeEnabled(),
    )
    fireEvent.click(screen.getByRole('button', { name: 'Paste table' }))
    await waitFor(() => expect(commands).toHaveLength(1))
    expect(commands[0]).toEqual({
      KerningMaintenance: {
        master: 'bold',
        vertical: false,
        expected_generation: 4,
        operation: {
          PasteKeepingExisting: {
            records: items.map(({ left, right, value }) => ({
              left,
              right,
              value,
            })),
          },
        },
      },
    })
  })

  it('keeps imported layout tables read-only', async () => {
    setTransport(
      createMockTransport({
        'kerning.pairs': () => ({
          status: { generation: 4, editable: false, reason: 'imported_layout' },
          total: items.length,
          items,
        }),
      }),
    )
    render(
      <KerningTable doc={1} master="regular" generation={4} vertical={false} />,
    )
    openTable()
    await screen.findByRole('table')
    expect(screen.getByRole('button', { name: 'Compress' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Clean Up' })).toBeDisabled()
    expect(
      screen.getByRole('button', { name: 'Rename public.kern1.A' }),
    ).toBeDisabled()
  })

  it('keeps horizontal and vertical clipboards separate', async () => {
    const view = render(
      <KerningTable doc={1} master="regular" generation={4} vertical={false} />,
    )
    openTable()
    fireEvent.click(await screen.findByRole('button', { name: 'Copy table' }))
    view.rerender(
      <KerningTable doc={1} master="regular" generation={4} vertical />,
    )
    await screen.findByRole('table')
    expect(screen.getByRole('button', { name: 'Paste table' })).toBeDisabled()
  })

  it('allows inspection and copying when the kerning mode disables mutations', async () => {
    render(
      <KerningTable
        doc={1}
        master="regular"
        generation={4}
        vertical={false}
        disabled
      />,
    )
    openTable()
    await screen.findByRole('table')
    expect(screen.getByRole('button', { name: 'Compress' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Copy table' })).toBeEnabled()
    expect(screen.getByRole('button', { name: 'Remove A / V' })).toBeDisabled()
  })
})
