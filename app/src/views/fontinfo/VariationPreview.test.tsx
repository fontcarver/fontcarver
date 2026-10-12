import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'

import { setTransport, type VariationPreview as Preview } from '../../api'
import { createMockTransport } from '../../test/mockTransport'
import { makeInstance } from '../../test/fontFixtures'

import type { DesignSpaceValue } from './designSpaceSession'
import { VariationPreview } from './VariationPreview'

/** A weight axis from 400 to 700 with two named instances. */
function state(): DesignSpaceValue {
  return {
    generation: 1,
    axes: [
      {
        tag: 'wght',
        name: 'Weight',
        min: 400,
        default: 400,
        max: 700,
        map: [],
        hidden: false,
      },
    ],
    masters: [],
    instances: [
      makeInstance({ name: 'Regular', location: { wght: 400 } }),
      makeInstance({ name: 'Bold', location: { wght: 700 } }),
    ],
    first_glyph: 'A',
  }
}

function preview(): Preview {
  return {
    generation: 1,
    path: 'M0 0L10 0L10 10Z',
    width: 500,
    vertical_width: null,
    vertical_origin: null,
    units_per_em: 1000,
    sources: [],
    shape_status: [],
    issues: [],
  }
}

/** Records the location of every preview request. */
function previewTransport() {
  const locations: Record<string, number>[] = []
  const transport = createMockTransport({
    'variation.preview': (params) => {
      locations.push({ ...params.location })
      return preview()
    },
  })
  return Object.assign(transport, { locations })
}

beforeEach(() => vi.restoreAllMocks())
afterEach(() => setTransport(undefined))

it('steps the weight axis in whole units so the keyboard reaches 500', async () => {
  const transport = previewTransport()
  setTransport(transport)
  const user = userEvent.setup()
  render(
    <VariationPreview doc={1} state={state()} onRefresh={() => undefined} />,
  )
  const scrubber = screen.getByRole('slider', { name: 'Scrub Weight' })
  // (max - min) / 1000 would be 0.3 and never land on a whole value.
  expect(scrubber).toHaveAttribute('step', '1')
  const field = screen.getByRole('textbox', { name: 'Weight' })
  expect(field).toHaveValue('400')
  // Shift multiplies the step by ten: ten presses reach exactly 500.
  field.focus()
  for (let i = 0; i < 10; i++) await user.keyboard('{Shift>}{ArrowUp}{/Shift}')
  await waitFor(() => expect(field).toHaveValue('500'))
  await waitFor(() => expect(transport.locations.at(-1)).toEqual({ wght: 500 }))
})

it('accepts a typed coordinate and clamps it to the axis range', async () => {
  const transport = previewTransport()
  setTransport(transport)
  const user = userEvent.setup()
  render(
    <VariationPreview doc={1} state={state()} onRefresh={() => undefined} />,
  )
  const field = screen.getByRole('textbox', { name: 'Weight' })
  await user.clear(field)
  await user.type(field, '650{Enter}')
  await waitFor(() => expect(transport.locations.at(-1)).toEqual({ wght: 650 }))
  await user.clear(field)
  await user.type(field, '9000{Enter}')
  await waitFor(() => expect(transport.locations.at(-1)).toEqual({ wght: 700 }))
})

it('resets one axis to its default without touching the others', async () => {
  const transport = previewTransport()
  setTransport(transport)
  const user = userEvent.setup()
  render(
    <VariationPreview doc={1} state={state()} onRefresh={() => undefined} />,
  )
  const reset = screen.getByRole('button', {
    name: 'Reset Weight to its default',
  })
  // Nothing to reset while the axis sits at its default.
  expect(reset).toBeDisabled()
  const field = screen.getByRole('textbox', { name: 'Weight' })
  await user.clear(field)
  await user.type(field, '700{Enter}')
  await waitFor(() => expect(reset).toBeEnabled())
  await user.click(reset)
  await waitFor(() => expect(field).toHaveValue('400'))
  await waitFor(() => expect(transport.locations.at(-1)).toEqual({ wght: 400 }))
})

it('refines the step so a fractional named location stays reachable', async () => {
  setTransport(previewTransport())
  const withWidth = state()
  withWidth.axes = [
    {
      tag: 'wdth',
      name: 'Width',
      min: 75,
      default: 100,
      max: 125,
      map: [],
      hidden: false,
    },
  ]
  withWidth.instances = [
    makeInstance({ name: 'SemiCondensed', location: { wdth: 87.5 } }),
  ]
  render(
    <VariationPreview doc={1} state={withWidth} onRefresh={() => undefined} />,
  )
  expect(screen.getByRole('slider', { name: 'Scrub Width' })).toHaveAttribute(
    'step',
    '0.5',
  )
  expect(screen.getByRole('textbox', { name: 'Width' })).toHaveValue('100.0')
})
