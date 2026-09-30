import { fireEvent, render, screen } from '@testing-library/react'
import { expect, it, vi } from 'vitest'
import type { VariationPreview } from '../../api'
import { VariationCompatibility } from './VariationCompatibility'

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, values?: Record<string, unknown>) =>
      key === 'variation.compatibility.source'
        ? `Show ${String(values?.name)}`
        : key,
  }),
}))

const preview: VariationPreview = {
  generation: 1,
  path: 'M0 0L40 0L40 80Z',
  width: 500,
  vertical_width: null,
  vertical_origin: null,
  units_per_em: 1000,
  sources: [
    { name: 'Regular', path: 'M0 0L40 0L40 80Z' },
    { name: 'Bold', path: 'M0 0L80 0L80 100Z' },
  ],
  issues: [],
}

it('independently selects source outlines and matching point links', () => {
  const { container } = render(
    <VariationCompatibility
      preview={preview}
      busy={false}
      glyph="A"
      onRepair={vi.fn()}
    />,
  )
  expect(container.querySelectorAll('[data-source-outline]')).toHaveLength(2)
  fireEvent.click(
    screen.getByRole('checkbox', { name: 'variation.compatibility.points' }),
  )
  expect(container.querySelectorAll('[data-correspondence]')).toHaveLength(3)
  fireEvent.click(screen.getByRole('checkbox', { name: 'Show Bold' }))
  expect(container.querySelectorAll('[data-source-outline]')).toHaveLength(1)
  expect(container.querySelectorAll('[data-correspondence]')).toHaveLength(0)
  expect(
    screen.getByText('variation.compatibility.selectSources'),
  ).toBeVisible()
  fireEvent.click(screen.getByRole('checkbox', { name: 'Show Bold' }))
  expect(container.querySelectorAll('[data-correspondence]')).toHaveLength(3)
  fireEvent.click(
    screen.getByRole('checkbox', { name: 'variation.showSources' }),
  )
  expect(container.querySelectorAll('[data-source-outline]')).toHaveLength(0)
  expect(container.querySelectorAll('[data-point-marker]')).toHaveLength(0)
})

it('explains differing topology without drawing invented correspondence', () => {
  const { container } = render(
    <VariationCompatibility
      preview={{
        ...preview,
        sources: [
          preview.sources[0]!,
          { name: 'Broken', path: 'M0 0Q40 0 40 80Z' },
        ],
        issues: ['Broken: point kinds differ'],
      }}
      busy={false}
      glyph="A"
      onRepair={vi.fn()}
    />,
  )
  fireEvent.click(
    screen.getByRole('checkbox', { name: 'variation.compatibility.points' }),
  )
  expect(screen.getByText('variation.compatibility.mismatch')).toBeVisible()
  expect(screen.getByText('Broken: point kinds differ')).toBeVisible()
  expect(container.querySelectorAll('[data-correspondence]')).toHaveLength(0)
  expect(container.querySelectorAll('[data-point-marker]')).toHaveLength(6)
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

it('bounds marker rendering and falls back to source outlines', () => {
  const { container } = render(
    <VariationCompatibility
      preview={{
        ...preview,
        sources: [
          preview.sources[0]!,
          { name: 'Large', path: 'M0 0' + 'L1 1'.repeat(800) },
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
  expect(container.querySelectorAll('[data-source-outline]')).toHaveLength(2)
  expect(container.querySelectorAll('[data-point-marker]')).toHaveLength(0)
})
