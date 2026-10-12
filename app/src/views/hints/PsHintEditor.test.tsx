import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, expect, it, vi } from 'vitest'
import { setTransport, type HintState } from '../../api'
import { makeLayer, makeMaster } from '../../test/fontFixtures'
import { createMockTransport, noChange } from '../../test/mockTransport'
import { PsHintEditor } from './PsHintEditor'

afterEach(() => setTransport(undefined))
const state = (): HintState => ({
  generation: 7,
  is_cid: false,
  cff_private_dict_index: 0,
  master: makeMaster(),
  hints: makeLayer().hints,
  layer: 'layer-1',
  path: 'M0 0H80V700H0Z',
  can_autohint_ps: true,
  units_per_em: 1000,
  contour_count: 1,
  flex_paths: [],
})

it('previews automatic hints without mutating and applies them with the original revision', async () => {
  const value = state(),
    close = vi.fn()
  const hints = {
    ...makeLayer().hints,
    v_stems: [{ start: 0, width: 80, ranges: [] }],
  }
  const transport = createMockTransport({
    'hints.state': () => value,
    'hints.analyze': () => ({
      generation: 7,
      hints: [['I', hints]],
      private: value.master.private_dict,
      cid_private: [],
    }),
    'hints.apply': () => noChange(),
  })
  setTransport(transport)
  render(
    <PsHintEditor
      doc={1}
      master="m1"
      glyph="I"
      onClose={close}
      onApplied={vi.fn()}
    />,
  )
  await userEvent.click(
    await screen.findByRole('button', { name: 'Analyze stems and zones' }),
  )
  await waitFor(() =>
    expect(screen.getByRole('textbox', { name: 'Width' })).toHaveValue('80'),
  )
  expect(transport.calls.some((call) => call.method === 'hints.apply')).toBe(
    false,
  )
  await userEvent.click(screen.getByRole('button', { name: 'Apply changes' }))
  await waitFor(() => expect(close).toHaveBeenCalledOnce())
  expect(transport.calls).toContainEqual(
    expect.objectContaining({
      method: 'hints.apply',
      params: expect.objectContaining({
        expected_generation: 7,
        commands: [{ Ps: { glyph: 'I', layer: 'layer-1', hints } }],
      }),
    }),
  )
})

it('keeps the editor open and preserves the draft when applying fails', async () => {
  const close = vi.fn()
  const transport = createMockTransport({
    'hints.state': state,
    'hints.apply': () => {
      throw new Error('Revision changed')
    },
  })
  setTransport(transport)
  render(
    <PsHintEditor
      doc={1}
      master="m1"
      glyph="I"
      onClose={close}
      onApplied={vi.fn()}
    />,
  )
  await screen.findByRole('button', { name: 'Analyze stems and zones' })
  await userEvent.click(screen.getAllByRole('button', { name: 'Add stem' })[0])
  await userEvent.click(screen.getByRole('button', { name: 'Apply changes' }))
  await screen.findByRole('alert')
  expect(close).not.toHaveBeenCalled()
  expect(screen.getByRole('textbox', { name: 'Width' })).toHaveValue('80')
})

it('does not analyze a background layer using another layer’s outlines', async () => {
  const transport = createMockTransport({
    'hints.state': () => ({ ...state(), can_autohint_ps: false }),
  })
  setTransport(transport)
  render(
    <PsHintEditor
      doc={1}
      master="m1"
      glyph="I"
      layer="background"
      onClose={vi.fn()}
      onApplied={vi.fn()}
    />,
  )
  expect(
    await screen.findByRole('button', { name: 'Analyze stems and zones' }),
  ).toBeDisabled()
  expect(
    screen.getByText(/Automatic analysis is available on master layers/),
  ).toBeVisible()
  expect(transport.calls.every((call) => call.method !== 'hints.analyze')).toBe(
    true,
  )
})

it('edits FD1 and assigns it to the glyph without replacing FD0', async () => {
  const value = {
    ...state(),
    is_cid: true,
    cff_private_dict_index: 0,
    master: makeMaster({
      cff_private_dicts: [{ ...makeMaster().private_dict, std_hw: 90 }],
    }),
  }
  const transport = createMockTransport({
    'hints.state': () => value,
    'hints.apply': () => noChange(),
  })
  setTransport(transport)
  render(
    <PsHintEditor
      doc={1}
      master="m1"
      glyph="I"
      onClose={vi.fn()}
      onApplied={vi.fn()}
    />,
  )
  await userEvent.selectOptions(
    await screen.findByRole('combobox', { name: 'Edit private dictionary' }),
    '1',
  )
  const width = screen.getByRole('spinbutton', { name: 'StdHW' })
  expect(width).toHaveValue(90)
  await userEvent.clear(width)
  await userEvent.type(width, '100')
  await userEvent.selectOptions(
    screen.getByRole('combobox', { name: 'Glyph private dictionary' }),
    '1',
  )
  expect(
    screen.getByRole('button', { name: 'Analyze stems and zones' }),
  ).toBeDisabled()
  await userEvent.click(screen.getByRole('button', { name: 'Apply changes' }))
  await waitFor(() =>
    expect(transport.calls).toContainEqual(
      expect.objectContaining({
        method: 'hints.apply',
        params: expect.objectContaining({
          expected_generation: 7,
          commands: [
            {
              CffPrivate: {
                master: 'm1',
                index: 1,
                private: { ...value.master.cff_private_dicts![0], std_hw: 100 },
              },
            },
            {
              CffPrivateAssignment: {
                master: 'm1',
                glyph: 'I',
                layer: 'layer-1',
                index: 1,
              },
            },
          ],
        }),
      }),
    ),
  )
})

it('appends a CID dictionary and keeps the draft when its apply fails', async () => {
  const close = vi.fn()
  const transport = createMockTransport({
    'hints.state': () => ({
      ...state(),
      is_cid: true,
      cff_private_dict_index: 0,
    }),
    'hints.apply': () => {
      throw new Error('Revision changed')
    },
  })
  setTransport(transport)
  render(
    <PsHintEditor
      doc={1}
      master="m1"
      glyph="I"
      onClose={close}
      onApplied={vi.fn()}
    />,
  )
  await userEvent.click(
    await screen.findByRole('button', { name: 'Add private dictionary' }),
  )
  expect(
    screen.getByRole('combobox', { name: 'Edit private dictionary' }),
  ).toHaveValue('1')
  await userEvent.type(screen.getByRole('spinbutton', { name: 'StdVW' }), '75')
  await userEvent.click(screen.getByRole('button', { name: 'Apply changes' }))
  await screen.findByRole('alert')
  expect(close).not.toHaveBeenCalled()
  expect(screen.getByRole('spinbutton', { name: 'StdVW' })).toHaveValue(75)
})

it('adds a flex hint and a dot section and applies them as hint data', async () => {
  const close = vi.fn()
  const transport = createMockTransport({
    'hints.state': () => ({ ...state(), contour_count: 2 }),
    'hints.apply': () => noChange(),
  })
  setTransport(transport)
  render(
    <PsHintEditor
      doc={1}
      master="m1"
      glyph="I"
      onClose={close}
      onApplied={vi.fn()}
    />,
  )
  await userEvent.click(
    await screen.findByRole('button', { name: 'Add flex hint' }),
  )
  await userEvent.click(screen.getByRole('button', { name: 'Add dot section' }))
  await userEvent.click(screen.getByRole('button', { name: 'Apply changes' }))
  await waitFor(() => expect(close).toHaveBeenCalledOnce())
  expect(transport.calls).toContainEqual(
    expect.objectContaining({
      method: 'hints.apply',
      params: expect.objectContaining({
        commands: [
          {
            Ps: {
              glyph: 'I',
              layer: 'layer-1',
              hints: {
                ...makeLayer().hints,
                flex_hints: [{ contour: 0, point: 0, depth: 50 }],
                dot_sections: [0],
              },
            },
          },
        ],
      }),
    }),
  )
})

it('reports a flex hint whose curve pair no longer resolves', async () => {
  setTransport(
    createMockTransport({
      'hints.state': () => ({
        ...state(),
        contour_count: 1,
        hints: {
          ...makeLayer().hints,
          flex_hints: [{ contour: 0, point: 4, depth: 50 }],
        },
        flex_paths: [''],
      }),
    }),
  )
  render(
    <PsHintEditor
      doc={1}
      master="m1"
      glyph="I"
      onClose={vi.fn()}
      onApplied={vi.fn()}
    />,
  )
  expect(await screen.findByText(/no curve pair at this point/)).toBeVisible()
})

it('discloses that the external TrueType hinter is absent', async () => {
  setTransport(
    createMockTransport({
      'hints.tt_autohinter': ({ options }) => ({
        installed: false,
        path: '',
        version: '',
        problem:
          (options.hinting_range_min ?? 2) >
          (options.hinting_range_max ?? 10000)
            ? 'ttfautohint hinting range minimum exceeds its maximum'
            : '',
      }),
    }),
  )
  const { TtAutohintFields, DEFAULT_TT_AUTOHINT } =
    await import('./TtAutohintFields')
  render(
    <TtAutohintFields
      options={DEFAULT_TT_AUTOHINT}
      busy={false}
      onChange={vi.fn()}
    />,
  )
  expect(
    await screen.findByText(/ttfautohint is not installed on PATH/),
  ).toBeVisible()
  const minimum = screen.getByRole('spinbutton', {
    name: 'Hinting range minimum (ppem)',
  })
  expect(minimum).toHaveValue(null)
  expect(minimum).toHaveAttribute('min', '2')
  expect(
    screen.getByRole('spinbutton', { name: 'Hinting limit (ppem)' }),
  ).toHaveAttribute('min', '0')
})

it('names the kernel’s own reason when an option would be refused', async () => {
  setTransport(
    createMockTransport({
      'hints.tt_autohinter': ({ options }) => ({
        installed: true,
        path: '/usr/bin/ttfautohint',
        version: 'ttfautohint 1.8.4',
        problem:
          (options.hinting_range_min ?? 2) >
          (options.hinting_range_max ?? 10000)
            ? 'ttfautohint hinting range minimum exceeds its maximum'
            : '',
      }),
    }),
  )
  const { TtAutohintFields, DEFAULT_TT_AUTOHINT } =
    await import('./TtAutohintFields')
  render(
    <TtAutohintFields
      options={{
        ...DEFAULT_TT_AUTOHINT,
        hinting_range_min: 40,
        hinting_range_max: 10,
      }}
      busy={false}
      onChange={vi.fn()}
    />,
  )
  expect(await screen.findByRole('alert')).toHaveTextContent(
    /hinting range minimum exceeds its maximum/,
  )
  expect(
    await screen.findByText(/ttfautohint found: ttfautohint 1\.8\.4/),
  ).toBeVisible()
})

it('gives every flex and dot-section remove button its own name', async () => {
  setTransport(
    createMockTransport({
      'hints.state': () => ({
        ...state(),
        contour_count: 2,
        hints: {
          ...makeLayer().hints,
          flex_hints: [
            { contour: 0, point: 0, depth: 50 },
            { contour: 1, point: 0, depth: 50 },
          ],
          dot_sections: [0, 1],
        },
        flex_paths: ['M0 0', 'M0 0'],
      }),
    }),
  )
  render(
    <PsHintEditor
      doc={1}
      master="m1"
      glyph="I"
      onClose={vi.fn()}
      onApplied={vi.fn()}
    />,
  )
  for (const name of [
    'Remove flex hint 1',
    'Remove flex hint 2',
    'Remove dot section 1',
    'Remove dot section 2',
  ]) {
    expect(await screen.findByRole('button', { name })).toBeVisible()
  }
})
