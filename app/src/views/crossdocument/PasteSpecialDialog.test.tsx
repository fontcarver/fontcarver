import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { setTransport, type GlyphCell } from '../../api'
import { useSessionStore } from '../../stores/sessionStore'
import {
  createMockTransport,
  makeSummary,
  noChange,
} from '../../test/mockTransport'
import { PasteSpecialDialog } from './PasteSpecialDialog'

const target = makeSummary({ doc: 1, family_name: 'Target' })
const source = makeSummary({ doc: 2, family_name: 'Source' })
const inventory = ['A', 'B'].map((name, index): GlyphCell => ({
  gid: index + 7,
  name,
  unicodes: [],
  category: '',
  sub_category: '',
  script: '',
  width: 500,
  bbox: null,
  path: null,
  color_label: null,
  export: true,
  has_components: false,
  is_empty: true,
  generation: 0,
}))
beforeEach(() => useSessionStore.setState({ docs: [target, source] }))
afterEach(() => {
  setTransport(undefined)
  useSessionStore.setState({ docs: [] })
})

it('reviews explicit source ordering, captures both generations and invalidates changed data scope', async () => {
  const close = vi.fn()
  const mock = createMockTransport({
    'glyphs.list': () => ({ total: 2, items: inventory }),
    'doc.paste_special': (params) => {
      expect(params.mappings).toEqual([
        { source: 8, target: 12 },
        { source: 7, target: 13 },
      ])
      if (!params.dry_run) {
        expect(params.expected_generation).toBe(12)
        expect(params.expected_source_generation).toBe(3)
        expect(params.content).toBe('layer')
      }
      return {
        generation: 12,
        source_generation: 3,
        pasted: ['X', 'Y'],
        placeholders: [],
        added_bases: ['base'],
        warnings: [],
        change: params.dry_run ? null : noChange(),
      }
    },
    'doc.summary': () => target,
  })
  setTransport(mock)
  render(
    <PasteSpecialDialog target={target} glyphs={[12, 13]} onClose={close} />,
  )
  const user = userEvent.setup()
  await user.type(screen.getByLabelText('Source glyph names'), 'B A')
  await user.selectOptions(
    screen.getByLabelText('Paste destination'),
    'selected',
  )
  const apply = screen.getByRole('button', { name: 'Apply transfer' })
  expect(apply).toBeDisabled()
  await user.click(screen.getByRole('button', { name: 'Preview transfer' }))
  expect(await screen.findByRole('status')).toHaveTextContent(
    '2 glyphs to paste',
  )
  expect(apply).toBeEnabled()
  await user.selectOptions(screen.getByLabelText('Data to paste'), 'layer')
  expect(apply).toBeDisabled()
  await user.click(screen.getByRole('button', { name: 'Preview transfer' }))
  await screen.findByRole('status')
  await user.click(apply)
  expect(close).toHaveBeenCalledOnce()
})

it('keeps stale transfers open and requires a new preview', async () => {
  const close = vi.fn()
  setTransport(
    createMockTransport({
      'glyphs.list': () => ({ total: 2, items: inventory }),
      'doc.paste_special': (params) => {
        if (!params.dry_run) throw new Error('source changed since preview')
        return {
          generation: 1,
          source_generation: 2,
          pasted: ['A'],
          placeholders: [],
          added_bases: [],
          warnings: [],
          change: null,
        }
      },
    }),
  )
  render(<PasteSpecialDialog target={target} glyphs={[]} onClose={close} />)
  const user = userEvent.setup()
  await user.type(screen.getByLabelText('Source glyph names'), 'A')
  await user.click(screen.getByRole('button', { name: 'Preview transfer' }))
  await screen.findByRole('status')
  await user.click(screen.getByRole('button', { name: 'Apply transfer' }))
  expect(await screen.findByRole('alert')).toHaveTextContent(
    'source changed since preview',
  )
  expect(close).not.toHaveBeenCalled()
  expect(screen.getByRole('button', { name: 'Apply transfer' })).toBeDisabled()
})

it('rejects mismatched selected destinations before any kernel mutation', async () => {
  const mock = createMockTransport({
    'glyphs.list': () => ({ total: 2, items: inventory }),
  })
  setTransport(mock)
  render(<PasteSpecialDialog target={target} glyphs={[12]} onClose={vi.fn()} />)
  const user = userEvent.setup()
  await user.type(screen.getByLabelText('Source glyph names'), 'A B')
  await user.selectOptions(
    screen.getByLabelText('Paste destination'),
    'components',
  )
  await user.click(screen.getByRole('button', { name: 'Preview transfer' }))
  expect(await screen.findByRole('alert')).toHaveTextContent(
    'source has 2 glyphs',
  )
  expect(mock.calls.every((entry) => entry.method === 'glyphs.list')).toBe(true)
})

it('names every placeholder it will create and how to find them again', async () => {
  const transport = createMockTransport({
    'glyphs.list': () => ({
      items: [{ ...inventory[0], name: 'aacute', gid: 11 }],
      total: 1,
    }),
    'doc.paste_special': () => ({
      generation: 5,
      source_generation: 2,
      pasted: ['aacute'],
      added_bases: ['acute'],
      placeholders: [
        {
          name: 'acute',
          source_base: 'acute',
          tag: 'missing-component-base',
        },
      ],
      warnings: [
        'Created empty nonexporting placeholder `acute`, tagged `missing-component-base`.',
      ],
      change: null,
    }),
  })
  setTransport(transport)
  render(<PasteSpecialDialog target={target} glyphs={[]} onClose={vi.fn()} />)
  const user = userEvent.setup()
  await user.selectOptions(
    screen.getByLabelText('Missing component bases'),
    'placeholder',
  )
  await user.type(screen.getByLabelText('Source glyph names'), 'aacute')
  await user.click(screen.getByRole('button', { name: 'Preview transfer' }))

  const review = await screen.findByRole('region', { name: 'Transfer review' })
  expect(review).toHaveTextContent('Nonexporting placeholders: acute')
  expect(review).toHaveTextContent('tags~"missing-component-base"')
})
