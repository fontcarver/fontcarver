import { fireEvent, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, expect, it, vi } from 'vitest'
import { setTransport, type Glyph, type Layer } from '../../../api'
import {
  createMockTransport,
  noChange,
  type MockHandlers,
} from '../../../test/mockTransport'
import { LayerEffectsEditor } from './LayerEffectsEditor'

const layer: Layer = {
  id: 'l',
  kind: { Master: 'm' },
  name: 'Regular',
  width: 600,
  vertical_width: null,
  vertical_origin: null,
  contours: [],
  components: [],
  anchors: [],
  guides: [],
  background_image: null,
  hints: { h_stems: [], v_stems: [], d_stems: [], flex: false, hint_masks: [] },
  tt_instructions: null,
  color_layers: null,
  user_data: null,
}
const result = {
  generation: 19,
  example: {
    glyph: 'A',
    master: 'Regular',
    before: 'M0 0L100 100',
    after: 'M0 0L100 0L100 100Z',
    before_width: 600,
    after_width: 600,
    bounds: [0, 0, 100, 100] as [number, number, number, number],
  },
  approximated_layers: [],
  change: null,
}
const storedEffect = {
  kind: 'variable_stroke' as const,
  nib: 'rectangle' as const,
  cap: 'square' as const,
  join: 'bevel' as const,
  miter_limit: 3,
  progression: 'smooth' as const,
  points: [
    { position: 0, width: 18, height: 24, angle: -15 },
    { position: 0.3, width: 40, height: 60, angle: 30 },
    { position: 1, width: 72, height: 32, angle: 70 },
  ],
}

afterEach(() => setTransport(undefined))

function setup(initialLayer = layer, handlers: MockHandlers = {}) {
  const close = vi.fn()
  const applied = vi.fn()
  const mock = createMockTransport({
    'effects.apply': (params) => ({
      ...result,
      change: params.dry_run ? null : noChange(),
    }),
    ...handlers,
  })
  setTransport(mock)
  render(
    <LayerEffectsEditor
      doc={7}
      glyph={{ name: 'A', layers: [initialLayer] } as Glyph}
      layer={initialLayer}
      upem={1000}
      onClose={close}
      onApplied={applied}
    />,
  )
  return { user: userEvent.setup(), mock, close, applied }
}

function point(index: number) {
  return within(screen.getByRole('group', { name: `Pen point ${index}` }))
}

function editPoint(index: number, name: string, value: number) {
  fireEvent.change(point(index).getByRole('spinbutton', { name }), {
    target: { value: String(value) },
  })
}

async function preview(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole('button', { name: 'Preview changes' }))
  await screen.findByText(
    'Preview ready. Apply or bake this reviewed revision.',
  )
}

it('reviews variable pen edits before sending them and invalidates changed parameters', async () => {
  const { user, mock, close, applied } = setup()
  await user.click(screen.getByRole('button', { name: 'Add variable stroke' }))
  editPoint(1, 'Pen width', 24)
  editPoint(1, 'Pen height', 18)
  editPoint(1, 'Pen angle (°)', -30)
  editPoint(2, 'Pen width', 80)
  editPoint(2, 'Pen height', 64)
  editPoint(2, 'Pen angle (°)', 45)
  await user.selectOptions(
    screen.getByRole('combobox', { name: 'Progression' }),
    'smooth',
  )
  expect(mock.calls).toHaveLength(0)
  expect(screen.getByRole('button', { name: 'Apply changes' })).toBeDisabled()
  expect(screen.getByRole('button', { name: 'Bake result' })).toBeDisabled()

  await preview(user)
  expect(mock.calls).toHaveLength(1)
  expect(mock.calls[0]?.params).toMatchObject({
    doc: 7,
    glyph: 'A',
    layer: 'l',
    dry_run: true,
    bake: false,
    expected_generation: null,
    effects: [
      {
        enabled: true,
        effect: {
          kind: 'variable_stroke',
          progression: 'smooth',
          points: [
            { position: 0, width: 24, height: 18, angle: -30 },
            { position: 1, width: 80, height: 64, angle: 45 },
          ],
        },
      },
    ],
  })

  editPoint(2, 'Pen width', 90)
  expect(screen.getByRole('button', { name: 'Apply changes' })).toBeDisabled()
  expect(screen.getByRole('button', { name: 'Bake result' })).toBeDisabled()
  expect(mock.calls).toHaveLength(1)
  await preview(user)
  await user.click(screen.getByRole('button', { name: 'Apply changes' }))
  expect(mock.calls).toHaveLength(3)
  expect(mock.calls[2]?.params).toMatchObject({
    dry_run: false,
    bake: false,
    expected_generation: 19,
    effects: [
      {
        effect: {
          points: [
            { position: 0, width: 24, height: 18, angle: -30 },
            { position: 1, width: 90, height: 64, angle: 45 },
          ],
        },
      },
    ],
  })
  expect(close).toHaveBeenCalledOnce()
  expect(applied).toHaveBeenCalledOnce()
})

it('anchors endpoint positions and adds points into the largest remaining gap', async () => {
  const { user, mock } = setup()
  await user.click(screen.getByRole('button', { name: 'Add variable stroke' }))
  expect(
    point(1).getByRole('spinbutton', { name: 'Position (0–1)' }),
  ).toBeDisabled()
  expect(
    point(2).getByRole('spinbutton', { name: 'Position (0–1)' }),
  ).toBeDisabled()
  expect(
    screen.getByRole('button', { name: 'Remove pen point 1' }),
  ).toBeDisabled()
  expect(
    screen.getByRole('button', { name: 'Remove pen point 2' }),
  ).toBeDisabled()

  await user.click(screen.getByRole('button', { name: 'Add pen point' }))
  expect(
    point(2).getByRole('spinbutton', { name: 'Position (0–1)' }),
  ).toHaveValue(0.5)
  expect(
    point(2).getByRole('spinbutton', { name: 'Position (0–1)' }),
  ).toBeEnabled()
  editPoint(2, 'Position (0–1)', 0.2)
  await user.click(screen.getByRole('button', { name: 'Add pen point' }))
  expect(
    point(3).getByRole('spinbutton', { name: 'Position (0–1)' }),
  ).toHaveValue(0.6)
  expect(
    point(4).getByRole('spinbutton', { name: 'Position (0–1)' }),
  ).toHaveValue(1)
  expect(
    screen.getByRole('button', { name: 'Remove pen point 4' }),
  ).toBeDisabled()

  await preview(user)
  await user.click(screen.getByRole('button', { name: 'Remove pen point 2' }))
  expect(screen.getByRole('button', { name: 'Apply changes' })).toBeDisabled()
  expect(
    screen.queryByRole('group', { name: 'Pen point 4' }),
  ).not.toBeInTheDocument()
  await preview(user)
  expect(mock.calls.at(-1)?.params).toMatchObject({
    effects: [
      {
        effect: {
          points: [{ position: 0 }, { position: 0.6 }, { position: 1 }],
        },
      },
    ],
  })
})

it('initializes stored variable pen parameters and bakes the reviewed effect stack', async () => {
  const initialLayer: Layer = {
    ...layer,
    effects: [{ enabled: true, effect: storedEffect }],
  }
  const { user, mock, applied } = setup(initialLayer)
  expect(
    point(2).getByRole('spinbutton', { name: 'Position (0–1)' }),
  ).toHaveValue(0.3)
  expect(point(2).getByRole('spinbutton', { name: 'Pen width' })).toHaveValue(
    40,
  )
  expect(point(2).getByRole('spinbutton', { name: 'Pen height' })).toHaveValue(
    60,
  )
  expect(
    point(2).getByRole('spinbutton', { name: 'Pen angle (°)' }),
  ).toHaveValue(30)
  expect(screen.getByRole('combobox', { name: 'Progression' })).toHaveValue(
    'smooth',
  )
  expect(screen.getByRole('option', { name: 'Linear' })).toBeInTheDocument()
  expect(screen.getByRole('option', { name: 'Smooth' })).toBeInTheDocument()
  await preview(user)
  await user.click(screen.getByRole('button', { name: 'Bake result' }))
  expect(mock.calls.at(-1)?.params).toMatchObject({
    effects: initialLayer.effects,
    dry_run: false,
    bake: true,
    expected_generation: 19,
  })
  expect(applied).toHaveBeenCalledOnce()
  expect(initialLayer.effects?.[0]?.effect).toEqual(storedEffect)
})

it('removes an existing variable effect only after reviewing the resulting stack', async () => {
  const { user, mock } = setup({
    ...layer,
    effects: [{ enabled: true, effect: storedEffect }],
  })
  await preview(user)
  await user.click(screen.getByRole('button', { name: 'Remove effect 1' }))
  expect(
    screen.queryByRole('group', { name: 'Pen point 1' }),
  ).not.toBeInTheDocument()
  expect(screen.getByRole('button', { name: 'Apply changes' })).toBeDisabled()
  expect(mock.calls).toHaveLength(1)
  await preview(user)
  await user.click(screen.getByRole('button', { name: 'Apply changes' }))
  expect(mock.calls.at(-1)?.params).toMatchObject({
    effects: [],
    dry_run: false,
    bake: false,
    expected_generation: 19,
  })
})

it('requires another review after changing progression and submits the selected mode', async () => {
  const { user, mock } = setup({
    ...layer,
    effects: [{ enabled: true, effect: storedEffect }],
  })
  await preview(user)
  await user.selectOptions(
    screen.getByRole('combobox', { name: 'Progression' }),
    'linear',
  )
  expect(screen.getByRole('button', { name: 'Apply changes' })).toBeDisabled()
  expect(screen.getByRole('button', { name: 'Bake result' })).toBeDisabled()
  expect(mock.calls).toHaveLength(1)
  expect(mock.calls[0]?.params).toMatchObject({
    effects: [{ effect: { progression: 'smooth' } }],
  })
  await preview(user)
  await user.click(screen.getByRole('button', { name: 'Apply changes' }))
  expect(mock.calls.at(-1)?.params).toMatchObject({
    dry_run: false,
    effects: [{ effect: { progression: 'linear' } }],
  })
})

it('keeps a rejected pen draft editable until corrected and successfully reviewed', async () => {
  const { user, mock, close, applied } = setup(
    { ...layer, effects: [{ enabled: true, effect: storedEffect }] },
    {
      'effects.apply': (params) => {
        const effect = params.effects[0]?.effect
        if (
          effect?.kind === 'variable_stroke' &&
          effect.points[1]?.position === 0
        )
          throw new Error('Pen points must have strictly ordered positions')
        return result
      },
    },
  )
  await preview(user)
  editPoint(2, 'Position (0–1)', 0)
  await user.click(screen.getByRole('button', { name: 'Preview changes' }))
  expect(await screen.findByRole('alert')).toHaveTextContent(
    'Pen points must have strictly ordered positions',
  )
  expect(screen.getByRole('button', { name: 'Apply changes' })).toBeDisabled()
  expect(screen.getByRole('button', { name: 'Bake result' })).toBeDisabled()
  await user.click(screen.getByRole('button', { name: 'Apply changes' }))
  expect(mock.calls).toHaveLength(2)
  expect(mock.calls.at(-1)?.params).toMatchObject({ dry_run: true })
  expect(close).not.toHaveBeenCalled()
  expect(applied).not.toHaveBeenCalled()
  expect(
    point(2).getByRole('spinbutton', { name: 'Position (0–1)' }),
  ).toBeEnabled()

  editPoint(2, 'Position (0–1)', 0.4)
  expect(screen.queryByRole('alert')).not.toBeInTheDocument()
  await preview(user)
  expect(screen.getByRole('button', { name: 'Apply changes' })).toBeEnabled()
  expect(mock.calls.at(-1)?.params).toMatchObject({
    effects: [
      {
        effect: {
          points: [{ position: 0 }, { position: 0.4 }, { position: 1 }],
        },
      },
    ],
  })
})
