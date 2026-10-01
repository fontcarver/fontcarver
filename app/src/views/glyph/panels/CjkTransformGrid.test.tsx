import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { setTransport, type ApplyParams, type Component } from '../../../api'
import { i18n } from '../../../i18n'
import catalog from '../../../i18n/en.json'
import { setCommandErrorReporter } from '../../../shell/commands'
import {
  makeCell,
  makeGlyph,
  makeGlyphDetail,
  makeLayer,
  makeLayerOutline,
  makeMaster,
} from '../../../test/fontFixtures'
import {
  createMockTransport,
  noChange,
  type MockTransport,
} from '../../../test/mockTransport'
import { buildScene } from '../glyphScene'

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

const scene = buildScene({
  detail: makeGlyphDetail(makeGlyph({ name: 'uni6797' })),
  cell: makeCell(),
  master: makeMaster(),
  masterId: 'm1',
  unitsPerEm: 1000,
})

function outlineOf(components: Component[] = [component]) {
  return makeLayerOutline(makeLayer(), {
    components: components.map((value) => ({
      component: value,
      path: 'M0 0L100 0L100 100Z',
      bbox: null,
    })),
  })
}

function applied(mock: MockTransport): ApplyParams[] {
  return mock.calls
    .filter((call) => call.method === 'doc.apply')
    .map((call) => call.params as ApplyParams)
}

function field(label: string): HTMLElement {
  return screen.getByRole('textbox', { name: label })
}

async function enter(input: HTMLElement, value: string) {
  const user = userEvent.setup()
  await user.clear(input)
  await user.type(input, `${value}{Enter}`)
}

beforeEach(() => {
  i18n.addResourceBundle('en', 'translation', catalog, true, true)
})

afterEach(() => {
  setTransport(undefined)
  setCommandErrorReporter(() => undefined)
})

describe('CJK component transform grid', () => {
  it('stages every field without touching the document, then applies one entry', async () => {
    const mock = createMockTransport({ 'doc.apply': () => noChange([42]) })
    setTransport(mock)
    const reload = vi.fn()
    const outline = outlineOf()
    render(
      <CjkTransformGrid
        target={target}
        scene={scene}
        outline={outline}
        reload={reload}
      />,
    )
    for (const [label, value] of [
      ['X for component 1 (uni6728)', '75'],
      ['Y for component 1 (uni6728)', '-20'],
      ['Scale X for component 1 (uni6728)', '-0.5'],
      ['Scale Y for component 1 (uni6728)', '1.5'],
      ['Shear X for component 1 (uni6728)', '0.5'],
      ['Shear Y for component 1 (uni6728)', '0.25'],
    ] as const) {
      await enter(field(label), value)
    }
    // Nothing has reached the kernel yet: the grid is a staged editor.
    expect(applied(mock)).toEqual([])
    expect(reload).not.toHaveBeenCalled()

    await userEvent
      .setup()
      .click(screen.getByRole('button', { name: 'Apply transforms' }))
    await waitFor(() => expect(reload).toHaveBeenCalledOnce())
    expect(applied(mock)).toEqual([
      {
        doc: 1,
        label: 'Transform CJK components',
        command: {
          Batch: [
            {
              SetComponent: {
                glyph: 'uni6797',
                layer: 'm1',
                index: 0,
                // Every other component property survives the edit.
                component: {
                  ...component,
                  transform: [-0.5, 0.25, 0.5, 1.5, 75, -20],
                },
              },
            },
          ],
        },
      },
    ])
    // The stored outline was never mutated in place.
    expect(outline.components[0]?.component.transform).toEqual([
      0.6, 0.1, -0.2, 0.8, 35, -15,
    ])
  })

  it('previews the staged transform before it is applied', async () => {
    setTransport(createMockTransport({ 'doc.apply': () => noChange([42]) }))
    render(
      <CjkTransformGrid
        target={target}
        scene={scene}
        outline={outlineOf([{ ...component, transform: [1, 0, 0, 1, 0, 0] }])}
        reload={vi.fn()}
      />,
    )
    const preview = screen.getByRole('img', {
      name: 'Component transform preview',
    })
    expect(preview.querySelector('path')).toHaveAttribute(
      'transform',
      'matrix(1 0 0 1 0 0)',
    )
    await enter(field('X for component 1 (uni6728)'), '250')
    // The preview follows the draft immediately, with no kernel round trip.
    expect(preview.querySelector('path')).toHaveAttribute(
      'transform',
      'matrix(1 0 0 1 250 0)',
    )
  })

  it('reverts drafts and forgets an edit returned to its stored value', async () => {
    const mock = createMockTransport({ 'doc.apply': () => noChange([42]) })
    setTransport(mock)
    render(
      <CjkTransformGrid
        target={target}
        scene={scene}
        outline={outlineOf()}
        reload={vi.fn()}
      />,
    )
    const apply = screen.getByRole('button', { name: 'Apply transforms' })
    const revert = screen.getByRole('button', { name: 'Revert transforms' })
    expect(apply).toBeDisabled()
    expect(revert).toBeDisabled()

    const x = field('X for component 1 (uni6728)')
    await enter(x, '420')
    expect(apply).toBeEnabled()
    // Typing the stored value back is not a change.
    await enter(x, '35')
    expect(apply).toBeDisabled()

    await enter(x, '420')
    await userEvent.setup().click(revert)
    expect(x).toHaveValue('35')
    expect(apply).toBeDisabled()
    expect(applied(mock)).toEqual([])
  })

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
        scene={scene}
        outline={outlineOf([component, second])}
        reload={vi.fn()}
      />,
    )
    const firstX = field('X for component 1 (uni6728)')
    const secondX = field('X for component 2 (uni6728)')
    expect(firstX).toHaveValue('35')
    expect(secondX).toHaveValue('300')
    await enter(secondX, '420')
    await userEvent
      .setup()
      .click(screen.getByRole('button', { name: 'Apply transforms' }))
    await waitFor(() => expect(applied(mock)).toHaveLength(1))
    // Only the edited row is written, and in component order.
    expect(applied(mock)[0]?.command).toEqual({
      Batch: [
        {
          SetComponent: {
            glyph: target.glyph,
            layer: target.layer,
            index: 1,
            component: { ...second, transform: [0.4, 0, 0, 0.9, 420, 10] },
          },
        },
      ],
    })
    expect(firstX).toHaveValue('35')
  })

  it('rejects invalid numbers without staging anything', async () => {
    const mock = createMockTransport({ 'doc.apply': () => noChange([42]) })
    setTransport(mock)
    render(
      <CjkTransformGrid
        target={target}
        scene={scene}
        outline={outlineOf()}
        reload={vi.fn()}
      />,
    )
    const input = field('Scale X for component 1 (uni6728)')
    await enter(input, '1/0')
    expect(input).toHaveValue('0.6')
    expect(input).toHaveAttribute('aria-invalid', 'true')
    expect(
      screen.getByRole('button', { name: 'Apply transforms' }),
    ).toBeDisabled()
    expect(applied(mock)).toEqual([])
  })

  it('refuses a collapsed scale with a reason instead of erasing the ink', async () => {
    const mock = createMockTransport({ 'doc.apply': () => noChange([42]) })
    setTransport(mock)
    render(
      <CjkTransformGrid
        target={target}
        scene={scene}
        outline={outlineOf()}
        reload={vi.fn()}
      />,
    )
    // A zero scale alone still shears; zeroing the whole first column is what
    // actually collapses the outline to a line.
    await enter(field('Shear Y for component 1 (uni6728)'), '0')
    await enter(field('Scale X for component 1 (uni6728)'), '0')
    expect(screen.getByRole('alert')).toHaveTextContent(
      'A scale of zero would erase the component',
    )
    expect(
      screen.getByRole('button', { name: 'Apply transforms' }),
    ).toBeDisabled()
    expect(applied(mock)).toEqual([])
  })

  it('keeps the drafts when the kernel refuses, so the edit can be retried', async () => {
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
          scene={scene}
          outline={{ ...outline }}
          reload={reload}
        />,
      ),
    )
    const view = render(
      <CjkTransformGrid
        target={target}
        scene={scene}
        outline={outline}
        reload={reload}
      />,
    )
    await enter(field('X for component 1 (uni6728)'), '420')
    const user = userEvent.setup()
    await user.click(screen.getByRole('button', { name: 'Apply transforms' }))
    await waitFor(() => expect(reload).toHaveBeenCalledOnce())
    expect(report).toHaveBeenCalledWith(error)
    // The identical component list keeps the draft, so Apply is still offered.
    expect(field('X for component 1 (uni6728)')).toHaveValue('420')
    await user.click(screen.getByRole('button', { name: 'Apply transforms' }))
    await waitFor(() => expect(applied(mock)).toHaveLength(2))
  })

  it('drops drafts when a reload changed the components', async () => {
    setTransport(createMockTransport({ 'doc.apply': () => noChange([42]) }))
    const view = render(
      <CjkTransformGrid
        target={target}
        scene={scene}
        outline={outlineOf()}
        reload={vi.fn()}
      />,
    )
    await enter(field('X for component 1 (uni6728)'), '420')
    view.rerender(
      <CjkTransformGrid
        target={target}
        scene={scene}
        outline={outlineOf([component, { ...component, anchor: null }])}
        reload={vi.fn()}
      />,
    )
    expect(field('X for component 1 (uni6728)')).toHaveValue('35')
    expect(
      screen.getByRole('button', { name: 'Apply transforms' }),
    ).toBeDisabled()
  })

  it('describes an empty component list without editable fields', () => {
    render(
      <CjkTransformGrid
        target={target}
        scene={scene}
        outline={outlineOf([])}
        reload={vi.fn()}
      />,
    )
    expect(screen.getByText('No components.')).toBeVisible()
    expect(screen.queryByRole('textbox')).not.toBeInTheDocument()
    expect(
      screen.queryByRole('button', { name: 'Apply transforms' }),
    ).not.toBeInTheDocument()
  })
})
