import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { setTransport, type Image, type ImageState } from '../../../api'
import {
  afterDocumentEdits,
  holdDocumentEdits,
} from '../../../editor/editSession'
import { i18n } from '../../../i18n'
import catalog from '../../../i18n/en.json'
import { initialEditorState, useEditorStore } from '../../../stores/editorStore'
import {
  createMockTransport,
  noChange,
  type MockHandlers,
} from '../../../test/mockTransport'
import { ImagePanel } from './ImagePanel'

const target = { doc: 1, glyph: 'A', layer: 'm1' }
const source: Image = {
  file_name: 'scan.png',
  transform: [1, 0, 0, 1, 12, 30],
  data: [137, 80, 78, 71],
  color: null,
  locked: false,
}
const state: ImageState = {
  image: source,
  generation: 7,
  png: source.data,
  width: 20,
  height: 30,
  error: null,
}
const traced = {
  generation: 7,
  source_hash: 'png-source-hash',
  contours: [],
  path: 'M12 30L32 30L32 60Z',
  bounds: [12, 30, 32, 60] as [number, number, number, number],
  change: null,
}

function setup(handlers: MockHandlers = {}) {
  const mock = createMockTransport({
    'image.state': () => state,
    'image.set': () => noChange([42]),
    'image.import': () => noChange([42]),
    'image.trace': () => traced,
    ...handlers,
  })
  setTransport(mock)
  return mock
}

beforeEach(() => {
  useEditorStore.setState(initialEditorState())
  i18n.addResourceBundle(
    'en',
    'translation',
    { imageWorkflow: catalog.imageWorkflow },
    true,
    true,
  )
})
afterEach(() => setTransport(undefined))

it('applies affine placement atomically against the loaded generation', async () => {
  const mock = setup()
  const reload = vi.fn()
  render(<ImagePanel target={target} glyphLocked={false} reload={reload} />)
  const x = await screen.findByRole('spinbutton', { name: 'Translate X' })
  fireEvent.change(x, { target: { value: '120' } })
  fireEvent.click(screen.getByRole('button', { name: 'Apply placement' }))
  await waitFor(() =>
    expect(mock.calls.find((c) => c.method === 'image.set')?.params).toEqual({
      ...target,
      expected_generation: 7,
      image: { ...source, transform: [1, 0, 0, 1, 120, 30] },
    }),
  )
  expect(reload).toHaveBeenCalledOnce()
})

it('previews and applies identical trace parameters and invalidates changed settings', async () => {
  const mock = setup()
  render(
    <ImagePanel target={target} glyphLocked={false} reload={() => undefined} />,
  )
  fireEvent.click(await screen.findByRole('button', { name: 'Preview trace' }))
  await screen.findByRole('img', { name: 'Traced outlines' })
  fireEvent.change(screen.getByRole('spinbutton', { name: 'Ink threshold' }), {
    target: { value: '160' },
  })
  expect(screen.getByRole('button', { name: 'Apply trace' })).toBeDisabled()
  fireEvent.click(screen.getByRole('button', { name: 'Preview trace' }))
  await screen.findByRole('img', { name: 'Traced outlines' })
  fireEvent.click(screen.getByRole('button', { name: 'Apply trace' }))
  await waitFor(() =>
    expect(mock.calls.filter((c) => c.method === 'image.trace')).toHaveLength(
      3,
    ),
  )
  const calls = mock.calls.filter((c) => c.method === 'image.trace')
  expect(calls[1]?.params).toMatchObject({
    threshold: 160,
    tolerance: 1,
    append: true,
    dry_run: true,
  })
  expect(calls[2]?.params).toEqual({
    ...(calls[1]?.params as object),
    dry_run: false,
    expected_generation: 7,
    expected_source_hash: 'png-source-hash',
  })
})

it('discards an in-flight preview when a document event changes the revision', async () => {
  let resolve!: (result: typeof traced) => void
  let current = state
  const mock = setup({
    'image.state': () => current,
    'image.trace': () =>
      new Promise((done) => {
        resolve = done
      }),
  })
  render(
    <ImagePanel target={target} glyphLocked={false} reload={() => undefined} />,
  )
  fireEvent.click(await screen.findByRole('button', { name: 'Preview trace' }))
  await waitFor(() => expect(resolve).toBeDefined())
  current = { ...state, generation: 8 }
  await act(async () => {
    mock.emit({
      doc: 1,
      generation: 8,
      change: noChange([42]),
      undo_label: null,
      redo_label: null,
      dirty: true,
    })
    resolve(traced)
  })
  expect(
    screen.queryByRole('img', { name: 'Traced outlines' }),
  ).not.toBeInTheDocument()
  expect(
    await screen.findByRole('button', { name: 'Apply trace' }),
  ).toBeDisabled()
})

it('defers PNG reloads during a point drag and drops previews from before its events', async () => {
  let resolve!: (result: typeof traced) => void
  const mock = setup({
    'image.trace': () =>
      new Promise((done) => {
        resolve = done
      }),
  })
  render(
    <ImagePanel target={target} glyphLocked={false} reload={() => undefined} />,
  )
  fireEvent.click(await screen.findByRole('button', { name: 'Preview trace' }))
  await waitFor(() => expect(resolve).toBeDefined())
  await act(async () => {
    useEditorStore.setState({ editing: true })
    for (const generation of [8, 9, 10])
      mock.emit({
        doc: 1,
        generation,
        change: noChange([42]),
        undo_label: null,
        redo_label: null,
        dirty: true,
      })
    resolve(traced)
  })
  expect(mock.calls.filter((c) => c.method === 'image.state')).toHaveLength(1)
  expect(
    screen.queryByRole('img', { name: 'Traced outlines' }),
  ).not.toBeInTheDocument()
  act(() => useEditorStore.setState({ editing: false }))
  await waitFor(() =>
    expect(mock.calls.filter((c) => c.method === 'image.state')).toHaveLength(
      2,
    ),
  )
})

it('lets a locked image unlock but blocks placement, replacement, and removal', async () => {
  const mock = setup({
    'image.state': () => ({ ...state, image: { ...source, locked: true } }),
  })
  render(
    <ImagePanel target={target} glyphLocked={false} reload={() => undefined} />,
  )
  expect(
    await screen.findByRole('spinbutton', { name: 'Translate X' }),
  ).toBeDisabled()
  expect(screen.getByRole('button', { name: 'Remove image' })).toBeDisabled()
  expect(screen.getByLabelText('Upload PNG')).toBeDisabled()
  fireEvent.click(
    screen.getByRole('checkbox', { name: 'Lock image placement' }),
  )
  await waitFor(() =>
    expect(
      mock.calls.find((c) => c.method === 'image.set')?.params,
    ).toMatchObject({ image: { locked: false }, expected_generation: 7 }),
  )
})

it('blocks all image writes when the glyph is locked', async () => {
  const mock = setup()
  render(<ImagePanel target={target} glyphLocked reload={() => undefined} />)
  expect(
    await screen.findByRole('checkbox', { name: 'Lock image placement' }),
  ).toBeDisabled()
  expect(screen.getByLabelText('Upload PNG')).toBeDisabled()
  expect(screen.getByRole('button', { name: 'Apply placement' })).toBeDisabled()
  expect(mock.calls.some((c) => c.method !== 'image.state')).toBe(false)
})

it('shows broken-link details and refreshes without mutating the source', async () => {
  let current = {
    ...state,
    image: { ...source, data: null },
    png: null,
    error: 'missing scan.png',
  } as ImageState
  const mock = setup({ 'image.state': () => current })
  render(
    <ImagePanel target={target} glyphLocked={false} reload={() => undefined} />,
  )
  expect(await screen.findByText('missing scan.png')).toBeVisible()
  current = state
  fireEvent.click(screen.getByRole('button', { name: 'Refresh image' }))
  await screen.findByText('Embedded PNG · 20 × 30 pixels')
  expect(mock.calls.every((c) => c.method === 'image.state')).toBe(true)
})

it('imports a linked server path and preserves its reviewed generation', async () => {
  const mock = setup({
    'image.state': () => ({ ...state, image: null, png: null }),
  })
  render(
    <ImagePanel target={target} glyphLocked={false} reload={() => undefined} />,
  )
  fireEvent.change(
    await screen.findByRole('textbox', { name: 'PNG path on this computer' }),
    { target: { value: '/fonts/images/scan.png' } },
  )
  fireEvent.click(screen.getByRole('button', { name: 'Import path' }))
  await waitFor(() =>
    expect(mock.calls.find((c) => c.method === 'image.import')?.params).toEqual(
      {
        ...target,
        expected_generation: 7,
        path: '/fonts/images/scan.png',
        data: null,
        file_name: 'scan.png',
        embed: false,
      },
    ),
  )
})

it('embeds uploaded PNG bytes and holds save while the file is still being read', async () => {
  let read!: (buffer: ArrayBuffer) => void
  const buffer = new Promise<ArrayBuffer>((resolve) => {
    read = resolve
  })
  const file = new File(['png'], 'source.png', { type: 'image/png' })
  Object.defineProperty(file, 'arrayBuffer', { value: () => buffer })
  const mock = setup()
  render(
    <ImagePanel target={target} glyphLocked={false} reload={() => undefined} />,
  )
  fireEvent.change(await screen.findByLabelText('Upload PNG'), {
    target: { files: [file] },
  })
  const saved = vi.fn()
  const saving = afterDocumentEdits(1, async () => {
    saved()
  })
  expect(saved).not.toHaveBeenCalled()
  await act(async () => {
    read(new Uint8Array([137, 80, 78, 71]).buffer)
    await saving
  })
  expect(mock.calls.find((c) => c.method === 'image.import')?.params).toEqual({
    ...target,
    expected_generation: 7,
    path: null,
    data: [137, 80, 78, 71],
    file_name: 'source.png',
    embed: true,
  })
  expect(saved).toHaveBeenCalledOnce()
})

it('rejects oversized browser uploads before reading or sending their bytes', async () => {
  const file = new File(['png'], 'large.png', { type: 'image/png' })
  Object.defineProperty(file, 'size', { value: 16 * 1024 * 1024 + 1 })
  const read = vi.fn()
  Object.defineProperty(file, 'arrayBuffer', { value: read })
  const mock = setup()
  render(
    <ImagePanel target={target} glyphLocked={false} reload={() => undefined} />,
  )
  fireEvent.change(await screen.findByLabelText('Upload PNG'), {
    target: { files: [file] },
  })
  expect(
    await screen.findByText('Choose a PNG no larger than 16 MiB.'),
  ).toBeVisible()
  expect(read).not.toHaveBeenCalled()
  expect(mock.calls.some((c) => c.method === 'image.import')).toBe(false)
})

it('invalidates review after a rejected trace apply without removing the source', async () => {
  const mock = setup({
    'image.trace': ({ dry_run }) => {
      if (dry_run) return traced
      throw new Error('linked image changed since preview')
    },
  })
  render(
    <ImagePanel target={target} glyphLocked={false} reload={() => undefined} />,
  )
  fireEvent.click(await screen.findByRole('button', { name: 'Preview trace' }))
  await screen.findByRole('img', { name: 'Traced outlines' })
  fireEvent.click(screen.getByRole('button', { name: 'Apply trace' }))
  expect(
    await screen.findByText('linked image changed since preview'),
  ).toBeVisible()
  expect(
    await screen.findByRole('button', { name: 'Apply trace' }),
  ).toBeDisabled()
  expect(screen.getByText('scan.png')).toBeVisible()
  expect(mock.calls.some((c) => c.method === 'image.set')).toBe(false)
})

it('waits for preceding edits and holds save until an image mutation finishes', async () => {
  let releaseEarlier!: () => void
  let releaseMutation!: () => void
  const earlier = new Promise<void>((resolve) => {
    releaseEarlier = resolve
  })
  const pending = new Promise<void>((resolve) => {
    releaseMutation = resolve
  })
  const mock = setup({
    'image.set': async () => {
      await pending
      return noChange([42])
    },
  })
  render(
    <ImagePanel target={target} glyphLocked={false} reload={() => undefined} />,
  )
  await screen.findByRole('button', { name: 'Remove image' })
  holdDocumentEdits(1, earlier)
  fireEvent.click(screen.getByRole('button', { name: 'Remove image' }))
  const saved = vi.fn()
  const saving = afterDocumentEdits(1, async () => {
    saved()
  })
  expect(mock.calls.some((c) => c.method === 'image.set')).toBe(false)
  await act(async () => releaseEarlier())
  await waitFor(() =>
    expect(mock.calls.some((c) => c.method === 'image.set')).toBe(true),
  )
  expect(saved).not.toHaveBeenCalled()
  await act(async () => {
    releaseMutation()
    await saving
  })
  expect(saved).toHaveBeenCalledOnce()
})
