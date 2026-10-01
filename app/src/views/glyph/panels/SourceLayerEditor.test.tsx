import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, expect, it, vi } from 'vitest'
import { setTransport } from '../../../api'
import { makeLayer, makeMaster } from '../../../test/fontFixtures'
import { createMockTransport, noChange } from '../../../test/mockTransport'
import { SourceLayerEditor } from './SourceLayerEditor'

afterEach(() => setTransport(undefined))
it('initializes supplemental coordinates from the mapped default when the master omits them', async () => {
  const applied = vi.fn()
  const master = makeMaster({ id: 'm', location: {} })
  const transport = createMockTransport({
    'variation.state': () => ({
      generation: 7,
      masters: [master],
      instances: [],
      first_glyph: 'A',
      axes: [
        {
          tag: [119, 103, 104, 116],
          name: 'Weight',
          min: 100,
          default: 400,
          max: 900,
          hidden: false,
          map: [
            [100, 0],
            [900, 100],
          ],
        },
      ],
    }),
    'variation.apply': (params) => {
      expect(params.expected_generation).toBe(7)
      expect(params.command).toMatchObject({
        Layer: { kind: { Brace: { master: 'm', location: { wght: 50 } } } },
      })
      return noChange()
    },
  })
  setTransport(transport)
  render(
    <SourceLayerEditor
      doc={1}
      glyph="A"
      source={makeLayer({ kind: { Master: 'm' } })}
      master="m"
      duplicate
      smartAxes={[]}
      onClose={() => {}}
      onApplied={applied}
    />,
  )
  const field = await screen.findByLabelText('wght')
  expect(field).toHaveValue('37.5')
  const user = userEvent.setup()
  await user.clear(field)
  await user.type(field, '50')
  await user.click(screen.getByRole('button', { name: 'Apply changes' }))
  await waitFor(() => expect(applied).toHaveBeenCalledOnce())
})

it('blocks empty and reversed conditions and submits valid inclusive design bounds', async () => {
  const applied = vi.fn()
  const transport = createMockTransport({
    'variation.state': () => ({
      generation: 9,
      masters: [makeMaster({ id: 'm' })],
      instances: [],
      first_glyph: 'A',
      axes: [
        {
          tag: [119, 103, 104, 116],
          name: 'Weight',
          min: 100,
          default: 400,
          max: 900,
          hidden: false,
          map: [
            [100, 0],
            [900, 100],
          ],
        },
      ],
    }),
    'variation.apply': () => noChange(),
  })
  setTransport(transport)
  render(
    <SourceLayerEditor
      doc={1}
      glyph="A"
      source={makeLayer({
        kind: {
          Bracket: {
            master: 'm',
            axis: [119, 103, 104, 116],
            min: 60,
            max: 30,
          },
        },
      })}
      master="m"
      duplicate={false}
      smartAxes={[]}
      onClose={() => {}}
      onApplied={applied}
    />,
  )
  await screen.findByText('The lower bound must not exceed the upper bound.')
  const apply = screen.getByRole('button', { name: 'Apply changes' })
  expect(apply).toBeDisabled()
  const user = userEvent.setup()
  await user.click(screen.getByRole('checkbox', { name: 'Inclusive maximum' }))
  expect(apply).toBeEnabled()
  await user.click(screen.getByRole('checkbox', { name: 'Inclusive minimum' }))
  expect(apply).toBeDisabled()
  expect(
    screen.getByText('Enable at least one condition bound.'),
  ).toBeInTheDocument()
  await user.click(screen.getByRole('checkbox', { name: 'Inclusive minimum' }))
  await user.click(apply)
  await waitFor(() => expect(applied).toHaveBeenCalledOnce())
  expect(
    transport.calls.find((call) => call.method === 'variation.apply')?.params,
  ).toMatchObject({
    expected_generation: 9,
    command: {
      Layer: {
        kind: {
          Bracket: {
            master: 'm',
            axis: [119, 103, 104, 116],
            min: 37.5,
            max: null,
          },
        },
      },
    },
  })
})

it('fills a virtual master position with mapped defaults and preserves an editable source name', async () => {
  const applied = vi.fn()
  const transport = createMockTransport({
    'variation.state': () => ({
      generation: 4,
      masters: [makeMaster({ id: 'm', location: { wght: 90, wdth: 140 } })],
      instances: [],
      first_glyph: 'A',
      virtual_masters: [{ name: 'Condensed light', location: { wdth: 75 } }],
      axes: [
        {
          tag: [119, 103, 104, 116],
          name: 'Weight',
          min: 100,
          default: 400,
          max: 900,
          hidden: false,
          map: [
            [100, 0],
            [900, 100],
          ],
        },
        {
          tag: [119, 100, 116, 104],
          name: 'Width',
          min: 50,
          default: 100,
          max: 150,
          hidden: false,
          map: [],
        },
      ],
    }),
    'variation.apply': () => noChange(),
  })
  setTransport(transport)
  render(
    <SourceLayerEditor
      doc={1}
      glyph="A"
      source={makeLayer({ kind: { Master: 'm' } })}
      master="m"
      duplicate
      smartAxes={[]}
      onClose={() => {}}
      onApplied={applied}
    />,
  )
  const preset = await screen.findByRole('combobox', {
    name: 'Virtual master position',
  })
  await userEvent.selectOptions(preset, '0')
  expect(screen.getByLabelText('wght')).toHaveValue('37.5')
  expect(screen.getByLabelText('wdth')).toHaveValue('75')
  const name = screen.getByRole('textbox', { name: 'Layer name' })
  expect(name).toHaveValue('Condensed light')
  await userEvent.clear(name)
  await userEvent.type(name, 'My source')
  await userEvent.click(screen.getByRole('button', { name: 'Apply changes' }))
  await waitFor(() => expect(applied).toHaveBeenCalledOnce())
  expect(
    transport.calls.find((call) => call.method === 'variation.apply')?.params,
  ).toMatchObject({
    command: {
      Layer: {
        name: 'My source',
        duplicate: true,
        kind: { Brace: { master: 'm', location: { wght: 37.5, wdth: 75 } } },
      },
    },
  })
})

it('edits a compound condition set and submits every range of it', async () => {
  const applied = vi.fn()
  const axes = [
    {
      tag: [119, 103, 104, 116] as [number, number, number, number],
      name: 'Weight',
      min: 100,
      default: 400,
      max: 900,
      hidden: false,
      map: [
        [100, 0],
        [900, 100],
      ] as [number, number][],
    },
    {
      tag: [119, 100, 116, 104] as [number, number, number, number],
      name: 'Width',
      min: 50,
      default: 100,
      max: 200,
      hidden: false,
      map: [] as [number, number][],
    },
  ]
  const transport = createMockTransport({
    'variation.state': () => ({
      generation: 4,
      masters: [makeMaster({ id: 'm' })],
      instances: [],
      first_glyph: 'A',
      axes,
    }),
    'variation.apply': () => noChange(),
  })
  setTransport(transport)
  render(
    <SourceLayerEditor
      doc={1}
      glyph="A"
      source={makeLayer({
        kind: {
          Bracket: {
            master: 'm',
            axis: [119, 103, 104, 116],
            min: 50,
            max: null,
            extra: [{ axis: [119, 100, 116, 104], min: 120 }],
          },
        },
      })}
      master="m"
      duplicate={false}
      smartAxes={[]}
      onClose={() => {}}
      onApplied={applied}
    />,
  )
  const user = userEvent.setup()
  // The stored second range is editable here, not only in the variation panel.
  expect(
    await screen.findByText('Active when [50‹wght, 120‹wdth] (inclusive).'),
  ).toBeVisible()
  const apply = screen.getByRole('button', { name: 'Apply changes' })
  await user.click(apply)
  await waitFor(() => expect(applied).toHaveBeenCalledOnce())
  expect(
    transport.calls.find((call) => call.method === 'variation.apply')?.params,
  ).toMatchObject({
    command: {
      Layer: {
        kind: {
          Bracket: {
            axis: [119, 103, 104, 116],
            min: 50,
            max: null,
            extra: [{ axis: [119, 100, 116, 104], min: 120 }],
          },
        },
      },
    },
  })
})

it('rejects a condition set that repeats an axis', async () => {
  const transport = createMockTransport({
    'variation.state': () => ({
      generation: 4,
      masters: [makeMaster({ id: 'm' })],
      instances: [],
      first_glyph: 'A',
      axes: [
        {
          tag: [119, 103, 104, 116] as [number, number, number, number],
          name: 'Weight',
          min: 100,
          default: 400,
          max: 900,
          hidden: false,
          map: [] as [number, number][],
        },
      ],
    }),
    'variation.apply': () => noChange(),
  })
  setTransport(transport)
  render(
    <SourceLayerEditor
      doc={1}
      glyph="A"
      source={makeLayer({
        kind: {
          Bracket: {
            master: 'm',
            axis: [119, 103, 104, 116],
            min: 200,
            max: null,
            extra: [{ axis: [119, 103, 104, 116], min: 300 }],
          },
        },
      })}
      master="m"
      duplicate={false}
      smartAxes={[]}
      onClose={() => {}}
      onApplied={vi.fn()}
    />,
  )
  expect(
    await screen.findByText(
      'Each axis can appear only once in a condition set.',
    ),
  ).toBeVisible()
  expect(screen.getByRole('button', { name: 'Apply changes' })).toBeDisabled()
})
