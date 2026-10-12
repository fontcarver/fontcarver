import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { expect, it } from 'vitest'
import type { FontCompareGlyphResult } from '../../api'
import { ComparisonOutline } from './ComparisonOutline'

const example: FontCompareGlyphResult = {
  name: 'A',
  bounds: [0, 0, 5000, 7000],
  target: {
    path: 'M0 0L250 700L500 0Z',
    bounds: [0, 0, 500, 700],
    width: 500,
    vertical_width: null,
    vertical_origin: null,
    upem: 1000,
    approximated: false,
  },
  source: {
    path: 'M0 0L2500 7000L5000 0Z',
    bounds: [0, 0, 5000, 7000],
    width: 5000,
    vertical_width: null,
    vertical_origin: null,
    upem: 10000,
    approximated: false,
  },
}
it('normalizes both source geometry and its viewport bounds', async () => {
  render(<ComparisonOutline outline={example} />)
  const svg = screen.getByRole('img', { name: 'Outline comparison for A' })
  expect(svg).toHaveAttribute('viewBox', '-80 -7080 5160 7160')
  await userEvent
    .setup()
    .click(screen.getByLabelText('Normalize source to target em size'))
  expect(svg).toHaveAttribute('viewBox', '-80 -1080 660 1160')
  expect(svg.querySelector('path[transform]')).toHaveAttribute(
    'transform',
    'scale(0.1)',
  )
})
it('keeps malformed zero-em previews finite', async () => {
  render(
    <ComparisonOutline
      outline={{
        ...example,
        source: example.source ? { ...example.source, upem: 0 } : null,
      }}
    />,
  )
  await userEvent
    .setup()
    .click(screen.getByLabelText('Normalize source to target em size'))
  expect(screen.getByRole('img').getAttribute('viewBox')).not.toMatch(
    /Infinity|NaN/,
  )
})
