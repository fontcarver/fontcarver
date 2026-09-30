import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'

import { setTransport, type ApplyParams, type Component } from '../../../api'
import { i18n } from '../../../i18n'
import catalog from '../../../i18n/en.json'
import { setCommandErrorReporter } from '../../../shell/commands'
import { makeLayer, makeLayerOutline } from '../../../test/fontFixtures'
import {
  createMockTransport,
  noChange,
  type MockTransport,
} from '../../../test/mockTransport'

import { CjkTransformGrid } from './CjkTransformGrid'

const target = { doc: 1, glyph: 'uni6797', layer: 'm1' }
const component: Component = {
  base: 'uni6728',
  transform: [0.6, 0.1, -0.2, 0.8, 35, -15],
  anchor: 'top',
  auto_align: true,
  use_my_metrics: true,
  round_to_grid: true,
  smart: { Width: 60 },
  point_match: [2, 3],
}

function outlineOf(components: Component[] = [component]) {
  return makeLayerOutline(makeLayer(), {
    components: components.map((value) => ({
      component: value,
      path: '',
      bbox: null,
    })),
  })
}

function applied(mock: MockTransport): ApplyParams[] {
  return mock.calls
    .filter((call) => call.method === 'doc.apply')
    .map((call) => call.params as ApplyParams)
}

beforeEach(() => {
  i18n.addResourceBundle('en', 'translation', catalog, true, true)
})

afterEach(() => {
  setTransport(undefined)
  setCommandErrorReporter(() => undefined)
})

it.each([
  ['Scale X', 0, -0.5],
  ['Shear Y', 1, 0.25],
  ['Shear X', 2, 0.5],
  ['Scale Y', 3, 1.5],
  ['X', 4, 75],
  ['Y', 5, -20],
] as const)(
  'commits %s once while preserving every component property',
  async (label, coefficient, value) => {
    const mock = createMockTransport({ 'doc.apply': () => noChange([42]) })
    setTransport(mock)
    const reload = vi.fn()
    const outline = outlineOf()
    render(
      <CjkTransformGrid target={target} outline={outline} reload={reload} />,
    )
    const user = userEvent.setup()
    const input = screen.getByRole('textbox', {
      name: `${label} for component 1 (uni6728)`,
    })
    await user.clear(input)
    await user.type(input, `${value}{Enter}`)
    await user.tab()
    await waitFor(() => expect(reload).toHaveBeenCalledOnce())
    const transform = [...component.transform]
    transform[coefficient] = value
    expect(applied(mock)).toEqual([
      {
        doc: 1,
        label: 'Set component',
        command: {
          SetComponent: {
            glyph: 'uni6797',
            layer: 'm1',
            index: 0,
            component: { ...component, transform },
          },
        },
      },
    ])
    expect(outline.components[0]?.component.transform).toEqual([
      0.6, 0.1, -0.2, 0.8, 35, -15,
    ])
  },
)

it('edits duplicate-base components by their independent row indexes', async () => {
  const mock = createMockTransport({ 'doc.apply': () => noChange([42]) })
  setTransport(mock)
  const second: Component = {
    ...component,
    transform: [0.4, 0, 0, 0.9, 300, 10],
    anchor: null,
    smart: { Width: 75 },
  }
  render(
    <CjkTransformGrid
      target={target}
      outline={outlineOf([component, second])}
      reload={vi.fn()}
    />,
  )
  const firstX = screen.getByRole('textbox', {
    name: 'X for component 1 (uni6728)',
  })
  const secondX = screen.getByRole('textbox', {
    name: 'X for component 2 (uni6728)',
  })
  expect(firstX).toHaveValue('35')
  expect(secondX).toHaveValue('300')
  const user = userEvent.setup()
  await user.clear(secondX)
  await user.type(secondX, '420{Enter}')
  await waitFor(() => expect(applied(mock)).toHaveLength(1))
  expect(applied(mock)[0]?.command).toEqual({
    SetComponent: {
      glyph: target.glyph,
      layer: target.layer,
      index: 1,
      component: { ...second, transform: [0.4, 0, 0, 0.9, 420, 10] },
    },
  })
  expect(firstX).toHaveValue('35')
})

it('rejects invalid numbers without applying a document command', async () => {
  const mock = createMockTransport({ 'doc.apply': () => noChange([42]) })
  setTransport(mock)
  const reload = vi.fn()
  render(
    <CjkTransformGrid target={target} outline={outlineOf()} reload={reload} />,
  )
  const user = userEvent.setup()
  const input = screen.getByRole('textbox', {
    name: 'Scale X for component 1 (uni6728)',
  })
  await user.clear(input)
  await user.type(input, '1/0{Enter}')
  expect(input).toHaveValue('0.6')
  expect(input).toHaveAttribute('aria-invalid', 'true')
  expect(applied(mock)).toEqual([])
  expect(reload).not.toHaveBeenCalled()
})

it('preserves an untouched fractional coefficient without writing on blur', async () => {
  const mock = createMockTransport({ 'doc.apply': () => noChange([42]) })
  setTransport(mock)
  render(
    <CjkTransformGrid
      target={target}
      outline={outlineOf([{ ...component, transform: [1 / 3, 0, 0, 1, 0, 0] }])}
      reload={vi.fn()}
    />,
  )
  const input = screen.getByRole('textbox', {
    name: 'Scale X for component 1 (uni6728)',
  })
  expect(input).toHaveValue(String(1 / 3))
  const user = userEvent.setup()
  await user.click(input)
  await user.tab()
  expect(applied(mock)).toEqual([])
})

it('reconciles a rejected value from a new outline and permits retry', async () => {
  const error = new Error('component edit rejected')
  const report = vi.fn()
  setCommandErrorReporter(report)
  const mock = createMockTransport({
    'doc.apply': () => {
      throw error
    },
  })
  setTransport(mock)
  const outline = outlineOf()
  const reload = vi.fn(() =>
    view.rerender(
      <CjkTransformGrid
        target={target}
        outline={{ ...outline }}
        reload={reload}
      />,
    ),
  )
  const view = render(
    <CjkTransformGrid target={target} outline={outline} reload={reload} />,
  )
  const user = userEvent.setup()
  const input = screen.getByRole('textbox', {
    name: 'X for component 1 (uni6728)',
  })
  await user.clear(input)
  await user.type(input, '420{Enter}')
  await waitFor(() => expect(reload).toHaveBeenCalledOnce())
  expect(report).toHaveBeenCalledWith(error)
  expect(input).toHaveValue('35')
  await user.clear(input)
  await user.type(input, '420{Enter}')
  await waitFor(() => expect(reload).toHaveBeenCalledTimes(2))
  expect(applied(mock)).toHaveLength(2)
  expect(input).toHaveValue('35')
})

it('describes an empty component list without editable fields', () => {
  render(
    <CjkTransformGrid
      target={target}
      outline={outlineOf([])}
      reload={vi.fn()}
    />,
  )
  expect(screen.getByText('No components.')).toBeVisible()
  expect(screen.queryByRole('textbox')).not.toBeInTheDocument()
})
