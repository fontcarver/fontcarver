import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import {
  setTransport,
  type FontCompareFamilyResult,
  type FontCompareResult,
} from '../../api'
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
const compatible = {
  family_name: true,
  units_per_em: true,
  axes: true,
  masters: true,
  glyph_set: true,
  master_metrics: true,
  metadata: true,
  kerning: true,
  layout: true,
}
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
  compatibility: { ...compatible, metadata: false, kerning: false },
  counts: {
    equal: 0,
    missing_target: 0,
    missing_source: 0,
    outlines: 1,
    metrics: 1,
    unicodes: 0,
    incompatible: 0,
    components: 0,
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
      components_changed: false,
      target_width: 500,
      source_width: 550,
      target_unicodes: [65],
      source_unicodes: [65],
      issues: [],
    },
  ],
  metadata: [
    {
      field: 'open_type_name_designer',
      target: 'Us',
      source: 'Reference designer',
    },
  ],
  kerning: {
    groups_only_target: [],
    groups_only_source: ['side1:public.kern1.A'],
    groups_changed: [],
    pairs_only_target: 0,
    pairs_only_source: 1,
    pairs_changed: 0,
    pairs_equal: 4,
    examples: [
      {
        left: '@public.kern1.A',
        right: 'B',
        vertical: false,
        target: null,
        source: -42,
      },
    ],
  },
  layout: {
    only_target: [],
    only_source: [],
    changed: [],
    classes_only_target: [],
    classes_only_source: [],
    classes_changed: [],
    source_of_truth_changed: false,
    feature_source_equal: true,
  },
}
const family: FontCompareFamilyResult = {
  compatible: false,
  members: [
    {
      doc: 1,
      family: 'Family',
      master: 'm1',
      master_name: 'Regular',
      reference: true,
      compatibility: compatible,
      counts: {
        equal: 0,
        missing_target: 0,
        missing_source: 0,
        outlines: 0,
        metrics: 0,
        unicodes: 0,
        incompatible: 0,
        components: 0,
      },
      examples: [],
      issue: null,
    },
    {
      doc: 2,
      family: 'Family',
      master: 'm2',
      master_name: 'Bold',
      reference: false,
      compatibility: { ...compatible, masters: false },
      counts: {
        equal: 3,
        missing_target: 0,
        missing_source: 1,
        outlines: 2,
        metrics: 0,
        unicodes: 0,
        incompatible: 0,
        components: 1,
      },
      examples: ['C'],
      issue: null,
    },
  ],
}
const merge = (dryRun: boolean) => ({
  change: dryRun ? null : noChange(),
  generation: 8,
  source_generation: 3,
  replaced: ['A'],
  kept: [],
  added_bases: [],
  adopted: ['kerning'],
  label: 'Merge fonts',
  warnings: ['Merging does not adopt OpenType layout source.'],
})

beforeEach(() => useSessionStore.setState({ docs: [target, source] }))
afterEach(() => {
  setTransport(undefined)
  useSessionStore.setState({ docs: [] })
})

it('shows every difference section and merges with per-conflict resolutions', async () => {
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
    'font.merge': (params) => {
      expect(params.glyphs).toEqual([{ name: 'A', choice: 'source' }])
      expect(params.adopt).toEqual({
        metadata: false,
        master_metrics: false,
        kerning: true,
      })
      expect(params.expected_generation).toBe(8)
      expect(params.expected_source_generation).toBe(3)
      return merge(params.dry_run === true)
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
  expect(results).toHaveTextContent('0 components')

  // Metadata, kerning and layout sections come from the same comparison.
  expect(
    within(results).getByRole('region', { name: 'Metadata differences' }),
  ).toHaveTextContent('Reference designer')
  const kerning = within(results).getByRole('region', {
    name: 'Kerning differences',
  })
  expect(kerning).toHaveTextContent('@public.kern1.A')
  expect(kerning).toHaveTextContent('4 equal')
  expect(
    within(results).getByRole('region', {
      name: 'OpenType layout differences',
    }),
  ).toHaveTextContent('Editable layout source matches.')

  await user.click(within(results).getByRole('button', { name: 'A' }))
  expect(
    await screen.findByRole('img', { name: 'Outline comparison for A' }),
  ).toBeVisible()

  await user.selectOptions(
    screen.getByLabelText('Resolve the conflict in A'),
    'source',
  )
  await user.click(
    screen.getByLabelText('Take reference kerning groups and pairs'),
  )
  await user.click(screen.getByRole('button', { name: 'Preview merge' }))
  const review = await screen.findByRole('region', {
    name: 'Conflict-resolved merge',
  })
  expect(review).toHaveTextContent('1 glyphs replaced or added')
  expect(review).toHaveTextContent('History entry: Merge fonts')
  await user.click(screen.getByRole('button', { name: 'Merge into target' }))
  expect(
    await screen.findByText(/Merged 1 glyphs as one undo entry/),
  ).toBeVisible()
  expect(
    mock.calls.filter((entry) => entry.method === 'font.merge'),
  ).toHaveLength(2)
})

it('refuses to preview a merge with no resolution or adoption chosen', async () => {
  setTransport(createMockTransport({ 'font.compare': () => comparison }))
  render(<CompareFontsDialog target={target} onClose={vi.fn()} />)
  const user = userEvent.setup()
  await user.click(screen.getByRole('button', { name: 'Compare fonts' }))
  await screen.findByRole('region', { name: 'Glyph differences' })
  expect(screen.getByRole('button', { name: 'Preview merge' })).toBeDisabled()
  await user.selectOptions(
    screen.getByLabelText('Resolve the conflict in A'),
    'target',
  )
  expect(
    screen.getByRole('button', { name: 'Preview merge' }),
  ).not.toBeDisabled()
})

it('compares every family master in one request and invalidates on master change', async () => {
  const mock = createMockTransport({
    'font.compare': () => comparison,
    'font.compare_family': (params) => {
      expect(params.all_masters).toBe(true)
      expect(params.docs).toEqual([2])
      return family
    },
  })
  setTransport(mock)
  render(<CompareFontsDialog target={target} onClose={vi.fn()} />)
  const user = userEvent.setup()
  await user.click(screen.getByRole('button', { name: 'Compare open family' }))
  const results = await screen.findByRole('region', {
    name: 'Family comparison',
  })
  expect(results).toHaveTextContent('Reference member')
  expect(results).toHaveTextContent('Bold')
  expect(results).toHaveTextContent('Masters')
  expect(results).toHaveTextContent('1 components')
  expect(
    within(results).getByText(
      'At least one family member differs from the reference member.',
    ),
  ).toBeVisible()
  expect(
    mock.calls.filter((entry) => entry.method === 'font.compare_family'),
  ).toHaveLength(1)

  await user.selectOptions(screen.getByLabelText('Target master'), 'm1')
  await user.click(screen.getByRole('button', { name: 'Compare fonts' }))
  await screen.findByRole('region', { name: 'Glyph differences' })
  expect(
    screen.queryByRole('region', { name: 'Family comparison' }),
  ).not.toBeInTheDocument()
})

it('opens a reference binary read-only and says so beside the source', async () => {
  const reference = makeSummary({ doc: 9, path: '/reference.otf' })
  const mock = createMockTransport({
    'font.open_reference': (params) => {
      expect(params.path).toBe('/fonts/Reference.otf')
      return {
        doc: 9,
        family: 'Reference',
        path: '/fonts/Reference.otf',
        units_per_em: 1000,
        masters: [{ id: 'm9', name: 'Regular' }],
        read_only: true,
        warnings: [],
      }
    },
    'doc.summary': () => reference,
  })
  setTransport(mock)
  render(<CompareFontsDialog target={target} onClose={vi.fn()} />)
  const user = userEvent.setup()
  await user.type(
    screen.getByLabelText('Reference font path'),
    '/fonts/Reference.otf',
  )
  await user.click(screen.getByRole('button', { name: 'Open reference' }))
  expect(
    await screen.findByText(/Opened Reference from \/fonts\/Reference.otf/),
  ).toBeVisible()
  expect(
    screen.getByText(/read-only reference \(\/fonts\/Reference.otf\)/),
  ).toBeVisible()
  // The reference is registered so it can be chosen as the comparison source.
  expect(
    within(screen.getByLabelText('Source font')).getByRole('option', {
      name: /reference.otf/i,
    }),
  ).toBeInTheDocument()
  expect(
    (screen.getByLabelText('Source font') as HTMLSelectElement).value,
  ).toBe('9')
})
