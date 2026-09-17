import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from '@testing-library/react'
import { beforeEach, afterEach, expect, it, vi } from 'vitest'

import { setTransport, type Command, type GlyphKerning } from '../../../api'
import {
  afterDocumentEdits,
  holdDocumentEdits,
} from '../../../editor/editSession'
import { i18n } from '../../../i18n'
import catalog from '../../../i18n/en.json'
import { setCommandErrorReporter } from '../../../shell/commands'
import {
  initialDocumentState,
  useDocumentStore,
} from '../../../stores/documentStore'
import {
  createMockTransport,
  noChange,
  type MockHandlers,
} from '../../../test/mockTransport'

import { KerningGroupsPanel } from './KerningGroupsPanel'

const en = catalog.kerningGroups
const status = { generation: 7, editable: true, reason: null }
const group = {
  name: 'public.kern1.A',
  member_count: 2,
  pair_count: 3,
  sample_members: ['A', 'a'],
  diagnostic: null,
}
const target = { doc: 1, glyph: 'A', layer: 'm1' }

function setup(extra: MockHandlers = {}) {
  const mock = createMockTransport({
    'kerning.glyph': () => ({
      status,
      side1: group.name,
      side2: null,
      diagnostics: [],
    }),
    'kerning.groups': ({ side }) => ({
      status,
      total: side === 'side1' ? 1 : 0,
      items: side === 'side1' ? [group] : [],
    }),
    'kerning.members': () => ({ status, total: 2, items: ['A', 'a'] }),
    'doc.apply': () => ({ ...noChange(), layout: true }),
    ...extra,
  })
  setTransport(mock)
  return mock
}

beforeEach(() => {
  i18n.addResourceBundle('en', 'translation', { kerningGroups: en }, true, true)
  useDocumentStore.setState(initialDocumentState())
  useDocumentStore.getState().ensure(1)
})
afterEach(() => {
  setTransport(undefined)
  setCommandErrorReporter(() => undefined)
})

async function manager() {
  await screen.findByText(group.name)
  fireEvent.click(
    screen.getByRole('button', { name: 'Manage First side groups' }),
  )
  fireEvent.click(
    await screen.findByRole('button', { name: `Inspect ${group.name}` }),
  )
  return screen.getByRole('group', { name: 'First side' })
}

it('creates and assigns a canonical group in one atomic command', async () => {
  const mock = setup()
  render(<KerningGroupsPanel target={target} />)
  const side = await manager()
  fireEvent.change(
    within(side).getByRole('textbox', { name: 'New First side group name' }),
    { target: { value: 'Round' } },
  )
  fireEvent.click(
    within(side).getByRole('button', { name: 'Create and assign' }),
  )
  await waitFor(() =>
    expect(
      mock.calls.find((call) => call.method === 'doc.apply')?.params,
    ).toMatchObject({
      doc: 1,
      command: {
        Batch: [
          {
            CreateKerningGroup: {
              side: 'side1',
              name: 'public.kern1.Round',
              members: [],
            },
          },
          {
            AssignKerningGroup: {
              side: 'side1',
              glyph: 'A',
              group: 'public.kern1.Round',
            },
          },
        ],
      },
    }),
  )
})

it('shows the group impact and requires explicit group-and-pairs removal', async () => {
  const mock = setup()
  render(<KerningGroupsPanel target={target} />)
  const side = await manager()
  expect(within(side).getByText('2 members')).toBeVisible()
  expect(
    within(side).getByText('3 pair records across all masters'),
  ).toBeVisible()
  fireEvent.click(within(side).getByRole('button', { name: 'Remove group…' }))
  expect(mock.calls.some((call) => call.method === 'doc.apply')).toBe(false)
  fireEvent.click(
    within(side).getByRole('button', { name: 'Remove group and pairs' }),
  )
  await waitFor(() =>
    expect(
      mock.calls.find((call) => call.method === 'doc.apply')?.params,
    ).toMatchObject({
      command: {
        RemoveKerningGroup: {
          side: 'side1',
          name: group.name,
          remove_pairs: true,
        },
      },
    }),
  )
})

it('uses create-only semantics when a new name already exists', async () => {
  const mock = setup({
    'doc.apply': ({ command }) => {
      expect(command).toHaveProperty('Batch.0.CreateKerningGroup')
      throw new Error('group name already exists')
    },
  })
  setCommandErrorReporter(() => undefined)
  render(<KerningGroupsPanel target={target} />)
  const side = await manager()
  fireEvent.change(
    within(side).getByRole('textbox', { name: 'New First side group name' }),
    { target: { value: group.name } },
  )
  fireEvent.click(
    within(side).getByRole('button', { name: 'Create and assign' }),
  )
  await screen.findByText(en.changeFailed)
  expect(screen.getByText(group.name)).toBeVisible()
  expect(mock.calls.filter((call) => call.method === 'doc.apply')).toHaveLength(
    1,
  )
})

it('renames the selected group with a canonical destination', async () => {
  const mock = setup()
  render(<KerningGroupsPanel target={target} />)
  const side = await manager()
  fireEvent.change(
    within(side).getByRole('textbox', { name: `Rename ${group.name}` }),
    { target: { value: 'Round' } },
  )
  fireEvent.click(within(side).getByRole('button', { name: 'Rename group' }))
  await waitFor(() =>
    expect(
      mock.calls.find((call) => call.method === 'doc.apply')?.params,
    ).toMatchObject({
      command: {
        RenameKerningGroup: {
          side: 'side1',
          from: group.name,
          to: 'public.kern1.Round',
        },
      },
    }),
  )
})

it('assigns an existing second-side group independently', async () => {
  const name = 'public.kern2.V'
  const mock = setup({
    'kerning.groups': () => ({ status, total: 1, items: [{ ...group, name }] }),
  })
  render(<KerningGroupsPanel target={target} />)
  await screen.findByText(group.name)
  fireEvent.click(
    screen.getByRole('button', { name: 'Manage Second side groups' }),
  )
  fireEvent.click(
    await screen.findByRole('button', { name: `Inspect ${name}` }),
  )
  fireEvent.click(screen.getByRole('button', { name: 'Assign A' }))
  await waitFor(() =>
    expect(
      mock.calls.find((call) => call.method === 'doc.apply')?.params,
    ).toMatchObject({
      command: {
        AssignKerningGroup: { side: 'side2', glyph: 'A', group: name },
      },
    }),
  )
})

it('reports a rejected change and restores the stored membership', async () => {
  const error = new Error('group name already exists')
  const report = vi.fn()
  setCommandErrorReporter(report)
  setup({
    'doc.apply': () => {
      throw error
    },
  })
  render(<KerningGroupsPanel target={target} />)
  await manager()
  fireEvent.click(
    screen.getByRole('button', { name: 'Remove A from this side' }),
  )
  expect(await screen.findByText(en.changeFailed)).toBeVisible()
  expect(screen.getByText(group.name)).toBeVisible()
  expect(report).toHaveBeenCalledWith(error)
})

it('restarts membership queries on document generation changes and ignores the older reply', async () => {
  let resolveOld!: (answer: GlyphKerning) => void
  let requests = 0
  const mock = setup({
    'kerning.glyph': () => {
      requests += 1
      return requests === 1
        ? new Promise((resolve) => {
            resolveOld = resolve
          })
        : {
            status: { ...status, generation: 8 },
            side1: 'public.kern1.New',
            side2: null,
            diagnostics: [],
          }
    },
  })
  render(<KerningGroupsPanel target={target} />)
  act(() =>
    mock.emit({
      doc: 1,
      generation: 8,
      change: { ...noChange(), layout: true },
      undo_label: 'Assign kerning group',
      redo_label: null,
      dirty: true,
    }),
  )
  await screen.findByText('public.kern1.New')
  await act(async () =>
    resolveOld({ status, side1: group.name, side2: null, diagnostics: [] }),
  )
  expect(screen.queryByText(group.name)).not.toBeInTheDocument()
})

it('carries the same generation on bounded continuation pages', async () => {
  const mock = setup({
    'kerning.groups': () => ({ status, total: 26, items: [group] }),
  })
  render(<KerningGroupsPanel target={target} />)
  await screen.findByText(group.name)
  fireEvent.click(
    screen.getByRole('button', { name: 'Manage First side groups' }),
  )
  fireEvent.click(await screen.findByRole('button', { name: 'Next page' }))
  await waitFor(() =>
    expect(
      mock.calls.some(
        (call) =>
          call.method === 'kerning.groups' &&
          (call.params as { offset: number }).offset === 25,
      ),
    ).toBe(true),
  )
  expect(
    mock.calls.filter((call) => call.method === 'kerning.groups').at(-1)
      ?.params,
  ).toMatchObject({ generation: 7, offset: 25, limit: 25 })
})

it('keeps unsupported kerning read-only and explains the reason', async () => {
  setup({
    'kerning.glyph': () => ({
      status: { ...status, editable: false, reason: 'imported_layout' },
      side1: group.name,
      side2: null,
      diagnostics: [],
    }),
  })
  render(<KerningGroupsPanel target={target} />)
  expect(
    await screen.findByText(
      'This font’s imported spacing rules cannot yet be edited.',
    ),
  ).toBeVisible()
  expect(
    screen.queryByRole('button', { name: /Manage/ }),
  ).not.toBeInTheDocument()
})

it('does not publish a membership reply for a previous glyph', async () => {
  let resolve!: (answer: GlyphKerning) => void
  setup({
    'kerning.glyph': ({ glyph }) =>
      glyph === 'A'
        ? new Promise((done) => {
            resolve = done
          })
        : { status, side1: 'public.kern1.V', side2: null, diagnostics: [] },
  })
  const view = render(<KerningGroupsPanel target={target} />)
  view.rerender(<KerningGroupsPanel target={{ ...target, glyph: 'V' }} />)
  await screen.findByText('public.kern1.V')
  await act(async () =>
    resolve({ status, side1: group.name, side2: null, diagnostics: [] }),
  )
  expect(screen.queryByText(group.name)).not.toBeInTheDocument()
})

it('waits for earlier edits and holds save behind a pending group command', async () => {
  let releaseEarlier!: () => void
  let releaseCommand!: () => void
  const earlier = new Promise<void>((resolve) => {
    releaseEarlier = resolve
  })
  const applied = new Promise<void>((resolve) => {
    releaseCommand = resolve
  })
  const commands: Command[] = []
  setup({
    'doc.apply': async ({ command }) => {
      commands.push(command)
      await applied
      return { ...noChange(), layout: true }
    },
  })
  render(<KerningGroupsPanel target={target} />)
  await manager()
  holdDocumentEdits(1, earlier)
  fireEvent.click(
    screen.getByRole('button', { name: 'Remove A from this side' }),
  )
  const saved = vi.fn()
  const saving = afterDocumentEdits(1, async () => {
    saved()
  })
  expect(commands).toEqual([])
  await act(async () => releaseEarlier())
  await waitFor(() => expect(commands).toHaveLength(1))
  expect(saved).not.toHaveBeenCalled()
  await act(async () => {
    releaseCommand()
    await saving
  })
  expect(saved).toHaveBeenCalledOnce()
})
