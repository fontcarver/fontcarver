import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { setTransport, type FontCompareResult } from '../../api'
import { useSessionStore } from '../../stores/sessionStore'
import {
  createMockTransport,
  makeSummary,
  noChange,
} from '../../test/mockTransport'
import { CompareFontsDialog } from './CompareFontsDialog'

const target = makeSummary({
  doc: 1,
  path: '/target.ttf',
  family_name: 'Family',
})
const source = makeSummary({
  doc: 2,
  path: '/source.ttf',
  family_name: 'Family',
})
const comparison: FontCompareResult = {
  generation: 8,
  source_generation: 3,
  total: 1,
  offset: 0,
  limit: 100,
  doc_family: 'Family',
  source_family: 'Family',
  doc_upem: 1000,
  source_upem: 1000,
  master: 'm1',
  source_master: 'm1',
  compatibility: {
    family_name: true,
    units_per_em: true,
    axes: true,
    masters: true,
    glyph_set: true,
    master_metrics: true,
  },
  counts: {
    equal: 0,
    missing_target: 0,
    missing_source: 0,
    outlines: 1,
    metrics: 1,
    unicodes: 0,
    incompatible: 0,
  },
  glyphs: [
    {
      name: 'A',
      source: 7,
      target: 2,
      status: 'different',
      outline_changed: true,
      metrics_changed: true,
      unicode_changed: false,
      compatible: true,
      target_width: 500,
      source_width: 550,
      target_unicodes: [65],
      source_unicodes: [65],
      issues: [],
    },
  ],
}
beforeEach(() => useSessionStore.setState({ docs: [target, source] }))
afterEach(() => {
  setTransport(undefined)
  useSessionStore.setState({ docs: [] })
})

it('shows visual differences and merges only reviewed source glyphs with revision guards', async () => {
  const mock = createMockTransport({
    'font.compare': () => comparison,
    'font.compare_glyph': () => ({
      name: 'A',
      bounds: [0, 0, 500, 700],
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
        path: 'M0 0L260 700L500 0Z',
        bounds: [0, 0, 500, 700],
        width: 550,
        vertical_width: null,
        vertical_origin: null,
        upem: 1000,
        approximated: false,
      },
    }),
    'doc.paste_special': (params) => {
      expect(params.mappings).toEqual([{ source: 7, target: null }])
      expect(params.expected_generation).toBe(8)
      expect(params.expected_source_generation).toBe(3)
      return {
        generation: 8,
        source_generation: 3,
        pasted: ['A'],
        added_bases: [],
        warnings: [],
        change: params.dry_run ? null : noChange(),
      }
    },
    'doc.summary': () => target,
  })
  setTransport(mock)
  render(<CompareFontsDialog target={target} onClose={vi.fn()} />)
  const user = userEvent.setup()
  await user.click(screen.getByRole('button', { name: 'Compare fonts' }))
  const results = await screen.findByRole('region', {
    name: 'Glyph differences',
  })
  expect(results).toHaveTextContent('1 outlines')
  await user.click(within(results).getByRole('button', { name: 'A' }))
  expect(
    await screen.findByRole('img', { name: 'Outline comparison for A' }),
  ).toBeVisible()
  await user.click(screen.getByLabelText('Select A for merge'))
  await user.click(
    screen.getByRole('button', { name: 'Preview selected merge' }),
  )
  await screen.findByRole('region', { name: 'Transfer review' })
  await user.click(screen.getByRole('button', { name: 'Merge into target' }))
  expect(await screen.findByRole('status')).toHaveTextContent('Merged 1 glyphs')
  expect(
    mock.calls.filter((entry) => entry.method === 'doc.paste_special'),
  ).toHaveLength(2)
})

it('compares every open family member and invalidates results when changing masters', async () => {
  useSessionStore.setState({
    docs: [target, source, makeSummary({ doc: 3, path: '/bold.ttf' })],
  })
  const mock = createMockTransport({ 'font.compare': () => comparison })
  setTransport(mock)
  render(<CompareFontsDialog target={target} onClose={vi.fn()} />)
  const user = userEvent.setup()
  await user.click(screen.getByRole('button', { name: 'Compare open family' }))
  const family = await screen.findByRole('region', {
    name: 'Family comparison',
  })
  expect(family).toHaveTextContent('source.ttf')
  expect(family).toHaveTextContent('bold.ttf')
  expect(mock.calls).toHaveLength(2)
  await user.selectOptions(screen.getByLabelText('Source font'), '3')
  expect(
    screen.queryByRole('region', { name: 'Family comparison' }),
  ).not.toBeInTheDocument()
})
