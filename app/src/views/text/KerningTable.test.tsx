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
  },
  { left: { glyph: 'A' }, right: { glyph: 'V' }, value: 0, diagnostic: null },
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

  it('requires scope confirmation before renaming every master reference', async () => {
    render(
      <KerningTable doc={1} master="regular" generation={4} vertical={false} />,
    )
    openTable()
    fireEvent.click(
      await screen.findByRole('button', { name: 'Rename public.kern1.A' }),
    )
    expect(screen.getByRole('dialog')).toHaveTextContent('all masters')
    fireEvent.change(screen.getByLabelText('New group name'), {
      target: { value: 'A.new' },
    })
    expect(commands).toHaveLength(0)
    fireEvent.click(
      screen.getByRole('button', { name: 'Rename in all masters' }),
    )
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
