import { fireEvent, render, screen } from '@testing-library/react'
import { expect, it, vi } from 'vitest'
import type {
  VariationPreview,
  VariationShape,
  VariationSourcePreview,
} from '../../api'
import { VariationCompatibility } from './VariationCompatibility'

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, values?: Record<string, unknown>) =>
      key === 'variation.compatibility.source'
        ? `Show ${String(values?.name)}`
        : key === 'variation.compatibility.moveEarlier'
          ? `Move earlier ${String(values?.index)}`
          : key === 'variation.compatibility.moveLater'
            ? `Move later ${String(values?.index)}`
            : key === 'variation.compatibility.dragShape'
              ? `Drag ${String(values?.index)}`
              : key,
  }),
}))

/** A closed square whose stored points are all on-curve. */
function square(x: number, y: number, size: number): VariationShape {
  return {
    index: 0,
    closed: true,
    path: `M${x} ${y}L${x} ${y + size}L${x + size} ${y + size}L${x + size} ${y}Z`,
    start: [x, y],
    center: [x + size / 2, y + size / 2],
    points: [
      [x, y],
      [x, y + size],
      [x + size, y + size],
      [x + size, y],
    ],
    on_curve: [true, true, true, true],
  }
}

function source(
  name: string,
  layer: string,
  shapes: VariationShape[],
): VariationSourcePreview {
  return {
    name,
    layer,
    path: shapes.map((shape) => shape.path).join(''),
    shapes: shapes.map((shape, index) => ({ ...shape, index })),
  }
}

const preview: VariationPreview = {
  generation: 1,
  path: 'M0 0L0 200L200 200L200 0Z',
  width: 500,
  vertical_width: null,
  vertical_origin: null,
  units_per_em: 1000,
  sources: [
    source('Regular', 'layer-a', [square(0, 0, 200), square(300, 0, 200)]),
    source('Bold', 'layer-b', [square(300, 0, 200), square(0, 0, 200)]),
  ],
  shape_status: [
    {
      index: 0,
      status: 'angle',
      segments: ['compatible', 'angle', 'compatible', 'compatible'],
      max_angle: 45,
    },
    {
      index: 1,
      status: 'compatible',
      segments: ['compatible', 'compatible', 'compatible', 'compatible'],
      max_angle: 0,
    },
  ],
  issues: [],
}

it('colours and numbers shapes by shape order in every source', () => {
  const { container } = render(
    <VariationCompatibility
      preview={preview}
      busy={false}
      glyph="A"
      onRepair={vi.fn()}
    />,
  )
  const outlines = [...container.querySelectorAll('[data-source-outline]')]
  expect(outlines).toHaveLength(4)
  // Shape 0 of both sources shares a colour class; shape 1 uses another.
  const colourOf = (source: string, shape: string) =>
    (
      outlines
        .find(
          (element) =>
            element.getAttribute('data-source-outline') === source &&
            element.getAttribute('data-shape') === shape,
        )
        ?.getAttribute('class') ?? ''
    )
      .split(/\s+/)
      .find((name) => name.includes('shape')) ?? ''
  expect(colourOf('Regular', '0')).not.toBe('')
  expect(colourOf('Regular', '0')).toEqual(colourOf('Bold', '0'))
  expect(colourOf('Regular', '0')).not.toEqual(colourOf('Regular', '1'))
  // A non-reference source is dashed, so the two are still distinguishable.
  expect(
    outlines
      .find((element) => element.getAttribute('data-source-outline') === 'Bold')
      ?.getAttribute('class'),
  ).toMatch(/secondary/)
  // One index label per shape of the reference source.
  expect(screen.getByText('1')).toBeInTheDocument()
  expect(screen.getByText('2')).toBeInTheDocument()
})

it('draws start-point and centre connectors for every shape', () => {
  const { container } = render(
    <VariationCompatibility
      preview={preview}
      busy={false}
      glyph="A"
      onRepair={vi.fn()}
    />,
  )
  const starts = [...container.querySelectorAll('[data-connector="start"]')]
  expect(starts).toHaveLength(2)
  expect(container.querySelectorAll('[data-connector="center"]')).toHaveLength(
    2,
  )
  // The reference's shape 0 starts at (0,0) and Bold's shape 0 at (300,0):
  // the crossing connector is what reveals the swapped order.
  expect(starts[0]?.getAttribute('points')).toBe('0,0 300,0')
  expect(starts[1]?.getAttribute('points')).toBe('300,0 0,0')
})

it('colours each segment with the status the kernel reported', () => {
  const { container } = render(
    <VariationCompatibility
      preview={preview}
      busy={false}
      glyph="A"
      onRepair={vi.fn()}
    />,
  )
  const segments = [...container.querySelectorAll('[data-segment-status]')]
  expect(segments).toHaveLength(8)
  expect(
    segments.filter(
      (segment) => segment.getAttribute('data-segment-status') === 'angle',
    ),
  ).toHaveLength(1)
  const incompatible = render(
    <VariationCompatibility
      preview={{
        ...preview,
        shape_status: [
          {
            index: 0,
            status: 'incompatible',
            segments: ['incompatible', 'incompatible'],
            max_angle: 0,
          },
        ],
        issues: ['Bold: point count differs'],
      }}
      busy={false}
      glyph="A"
      onRepair={vi.fn()}
    />,
  )
  expect(
    incompatible.container.querySelectorAll(
      '[data-segment-status="incompatible"]',
    ).length,
  ).toBeGreaterThan(0)
  expect(screen.getByText('Bold: point count differs')).toBeVisible()
})

it('swaps shape order in one source by dragging one handle onto another', () => {
  const onReorder = vi.fn()
  const { container } = render(
    <VariationCompatibility
      preview={preview}
      busy={false}
      glyph="A"
      onRepair={vi.fn()}
      onReorder={onReorder}
    />,
  )
  const canvas = container.querySelector('svg')!
  canvas.getBoundingClientRect = () =>
    ({ left: 0, top: 0, width: 1000, height: 1000 }) as DOMRect
  const handles = [...container.querySelectorAll('[data-shape-handle]')]
  expect(handles).toHaveLength(2)
  fireEvent.pointerDown(handles[0]!)
  // Drop over the second shape's centre: the canvas maps client to font units.
  fireEvent.pointerUp(canvas, { clientX: 500, clientY: 500 })
  expect(onReorder).toHaveBeenCalledOnce()
  expect(onReorder.mock.calls[0]).toEqual(['layer-a', [1, 0]])
})

it('offers a keyboard alternative that reorders the chosen source only', () => {
  const onReorder = vi.fn()
  render(
    <VariationCompatibility
      preview={preview}
      busy={false}
      glyph="A"
      onRepair={vi.fn()}
      onReorder={onReorder}
    />,
  )
  fireEvent.change(
    screen.getByRole('combobox', {
      name: 'variation.compatibility.reorderSource',
    }),
    { target: { value: '1' } },
  )
  fireEvent.click(screen.getByRole('button', { name: 'Move earlier 2' }))
  expect(onReorder.mock.calls[0]).toEqual(['layer-b', [1, 0]])
  expect(screen.getByRole('button', { name: 'Move earlier 1' })).toBeDisabled()
  expect(screen.getByRole('button', { name: 'Move later 2' })).toBeDisabled()
})

it('never reorders while the preview is busy', () => {
  const onReorder = vi.fn()
  render(
    <VariationCompatibility
      preview={preview}
      busy
      glyph="A"
      onRepair={vi.fn()}
      onReorder={onReorder}
    />,
  )
  const button = screen.getByRole('button', { name: 'Move later 1' })
  expect(button).toBeDisabled()
  fireEvent.click(button)
  expect(onReorder).not.toHaveBeenCalled()
})

it('marks a component shape with its base glyph', () => {
  const componentShape: VariationShape = {
    ...square(0, 0, 100),
    component: 'A',
    points: [],
    on_curve: [],
  }
  render(
    <VariationCompatibility
      preview={{
        ...preview,
        sources: [source('Regular', 'layer-a', [componentShape])],
        shape_status: [
          {
            index: 0,
            component: 'A',
            status: 'compatible',
            segments: [],
            max_angle: 0,
          },
        ],
      }}
      busy={false}
      glyph="Aacute"
      onRepair={vi.fn()}
      onReorder={vi.fn()}
    />,
  )
  expect(
    screen.getByText('variation.compatibility.componentLabel'),
  ).toBeInTheDocument()
})

it('delegates repair and prevents repair while a preview or operation is busy', () => {
  const onRepair = vi.fn()
  const { rerender } = render(
    <VariationCompatibility
      preview={preview}
      busy={false}
      glyph="A"
      onRepair={onRepair}
    />,
  )
  fireEvent.click(screen.getByRole('button', { name: 'variation.repair' }))
  expect(onRepair).toHaveBeenCalledOnce()
  rerender(
    <VariationCompatibility
      preview={preview}
      busy={true}
      glyph="A"
      onRepair={onRepair}
    />,
  )
  expect(
    screen.getByRole('button', { name: 'variation.repair' }),
  ).toBeDisabled()
  rerender(
    <VariationCompatibility
      preview={null}
      busy={false}
      glyph=""
      onRepair={onRepair}
    />,
  )
  expect(
    screen.getByRole('button', { name: 'variation.repair' }),
  ).toBeDisabled()
  expect(screen.getByText('variation.compatibility.pending')).toBeVisible()
})

it('bounds marker rendering and still shows the source outlines', () => {
  const huge: VariationShape = {
    ...square(0, 0, 200),
    points: Array.from({ length: 1300 }, (_, index) => [index, index]),
    on_curve: Array.from({ length: 1300 }, () => true),
  }
  const { container } = render(
    <VariationCompatibility
      preview={{
        ...preview,
        sources: [source('Large', 'layer-a', [huge])],
        shape_status: [
          { index: 0, status: 'compatible', segments: [], max_angle: 0 },
        ],
      }}
      busy={false}
      glyph="A"
      onRepair={vi.fn()}
    />,
  )
  fireEvent.click(
    screen.getByRole('checkbox', { name: 'variation.compatibility.points' }),
  )
  expect(screen.getByText('variation.compatibility.pointLimit')).toBeVisible()
  expect(container.querySelectorAll('[data-source-outline]')).toHaveLength(1)
  expect(container.querySelectorAll('[data-point-marker]')).toHaveLength(0)
  expect(container.querySelectorAll('[data-segment-status]')).toHaveLength(0)
})

it('hides a source from the overlay without losing the others', () => {
  const { container } = render(
    <VariationCompatibility
      preview={preview}
      busy={false}
      glyph="A"
      onRepair={vi.fn()}
    />,
  )
  fireEvent.click(screen.getByRole('checkbox', { name: 'Show Bold' }))
  expect(container.querySelectorAll('[data-source-outline]')).toHaveLength(2)
  expect(container.querySelectorAll('[data-connector="start"]')).toHaveLength(0)
  fireEvent.click(
    screen.getByRole('checkbox', { name: 'variation.showSources' }),
  )
  expect(container.querySelectorAll('[data-source-outline]')).toHaveLength(0)
})
