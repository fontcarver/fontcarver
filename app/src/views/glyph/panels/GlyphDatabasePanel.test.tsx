import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { setTransport, type GlyphDataState } from '../../../api'
import {
  initialDocumentState,
  useDocumentStore,
} from '../../../stores/documentStore'
import {
  makeCell,
  makeGlyph,
  makeGlyphDetail,
  makeLayer,
  makeLayerOutline,
  makeMaster,
} from '../../../test/fontFixtures'
import { createMockTransport, noChange } from '../../../test/mockTransport'
import { ApiCallError } from '../../../transport'
import { buildScene } from '../glyphScene'
import { useUiStore } from '../../../stores/uiStore'
import { GlyphDatabasePanel } from './GlyphDatabasePanel'
import type { PanelContext } from './panelContext'

function context(overrides: Partial<PanelContext> = {}): PanelContext {
  const glyph = makeGlyph({ name: 'aacute' })
  return {
    target: { doc: 1, glyph: glyph.name, layer: 'layer-1' },
    gid: 42,
    glyph,
    cell: makeCell(),
    scene: buildScene({
      detail: makeGlyphDetail(glyph),
      cell: makeCell(),
      master: makeMaster(),
      masterId: 'm1',
      unitsPerEm: 1000,
    }),
    outline: makeLayerOutline(makeLayer()),
    master: 'm1',
    masterGuides: [],
    reload: vi.fn(),
    ...overrides,
  }
}

function database(overrides: Partial<GlyphDataState> = {}): GlyphDataState {
  return {
    generation: 3,
    custom: true,
    overlay_json: '{"version":1,"glyphs":{}}',
    info: {
      name: 'aacute',
      unicodes: [0xe1],
      category: 'Letter',
      sub_category: 'Lowercase',
      script: 'latin',
      production_name: 'aacute',
      anchors: ['top', 'bottom'],
      composition: ['a', 'acutecomb'],
    },
    ...overrides,
  }
}

beforeEach(() => {
  useUiStore.setState({
    sections: { 'glyph-database': true, 'glyph-database-custom': true },
  })
  useDocumentStore.setState(initialDocumentState())
  useDocumentStore.getState().ensure(1)
})
afterEach(() => {
  setTransport(undefined)
  useUiStore.setState({ sections: {} })
})

it('keeps advanced data closed initially and remembers each disclosure without losing a draft', async () => {
  useUiStore.setState({ sections: {} })
  setTransport(createMockTransport({ 'glyphdata.state': () => database() }))
  const props = context()
  const { rerender } = render(<GlyphDatabasePanel {...props} />)
  const databaseToggle = screen.getByText('Glyph database')
  expect(databaseToggle.closest('details')).not.toHaveAttribute('open')
  await userEvent.click(databaseToggle)
  await waitFor(() =>
    expect(useUiStore.getState().sections['glyph-database']).toBe(true),
  )
  const customToggle = screen.getByText('Custom glyph data')
  expect(customToggle.closest('details')).not.toHaveAttribute('open')
  await userEvent.click(customToggle)
  const editor = await screen.findByRole('textbox', {
    name: 'Custom glyph data JSON',
  })
  await waitFor(() => expect(editor).toBeEnabled())
  fireEvent.change(editor, {
    target: { value: '{"version":1,"glyphs":{"A":{}}}' },
  })
  await userEvent.click(databaseToggle)
  await waitFor(() =>
    expect(useUiStore.getState().sections['glyph-database']).toBe(false),
  )
  expect(useUiStore.getState().sections['glyph-database-custom']).toBe(true)
  rerender(<GlyphDatabasePanel {...props} reload={vi.fn()} />)
  await userEvent.click(databaseToggle)
  expect(
    await screen.findByRole('textbox', { name: 'Custom glyph data JSON' }),
  ).toHaveValue('{"version":1,"glyphs":{"A":{}}}')
})

it('shows effective defaults and updates only the current glyph', async () => {
  const transport = createMockTransport({
    'glyphdata.state': () => database(),
    'glyphdata.update': () => noChange([42]),
  })
  setTransport(transport)
  const props = context()
  render(<GlyphDatabasePanel {...props} />)
  expect(await screen.findByText('top, bottom')).toBeVisible()
  expect(screen.getByText('a + acutecomb')).toBeVisible()
  expect(screen.getByText('Custom override')).toBeVisible()
  await userEvent.click(
    screen.getByRole('button', { name: 'Update Glyph Info' }),
  )
  await waitFor(() => expect(props.reload).toHaveBeenCalledOnce())
  expect(transport.calls).toContainEqual({
    method: 'glyphdata.update',
    params: {
      doc: 1,
      glyphs: [42],
      expected_generation: 3,
      label: 'Update Glyph Info',
    },
  })
})

it('saves reviewed JSON with the draft generation and rejects invalid JSON locally', async () => {
  const transport = createMockTransport({
    'glyphdata.state': () => database(),
    'glyphdata.set_overlay': () => noChange(),
  })
  setTransport(transport)
  render(<GlyphDatabasePanel {...context()} />)
  const editor = await screen.findByRole('textbox', {
    name: 'Custom glyph data JSON',
  })
  await waitFor(() => expect(editor).toHaveValue('{"version":1,"glyphs":{}}'))
  fireEvent.change(editor, { target: { value: '{invalid' } })
  await userEvent.click(
    screen.getByRole('button', { name: 'Save custom data' }),
  )
  expect(await screen.findByRole('alert')).toHaveTextContent('Enter valid JSON')
  expect(
    transport.calls.some(({ method }) => method === 'glyphdata.set_overlay'),
  ).toBe(false)
  const overlay = '{"version":1,"glyphs":{"aacute":{"anchors":["top"]}}}'
  fireEvent.change(editor, { target: { value: overlay } })
  await userEvent.click(
    screen.getByRole('button', { name: 'Save custom data' }),
  )
  await waitFor(() =>
    expect(transport.calls).toContainEqual({
      method: 'glyphdata.set_overlay',
      params: {
        doc: 1,
        expected_generation: 3,
        overlay_json: overlay,
        label: 'Edit custom glyph data',
      },
    }),
  )
})

it('imports a JSON file into a draft without saving it', async () => {
  const transport = createMockTransport({ 'glyphdata.state': () => database() })
  setTransport(transport)
  render(<GlyphDatabasePanel {...context()} />)
  await screen.findByText('top, bottom')
  const overlay = '{"version":1,"glyphs":{"a":{"category":"Letter"}}}'
  const file = new File([overlay], 'glyphs.json', { type: 'application/json' })
  Object.defineProperty(file, 'text', { value: async () => overlay })
  await userEvent.upload(screen.getByLabelText('Import JSON file'), file)
  await waitFor(() =>
    expect(
      screen.getByRole('textbox', { name: 'Custom glyph data JSON' }),
    ).toHaveValue(overlay),
  )
  expect(
    transport.calls.some(({ method }) => method === 'glyphdata.set_overlay'),
  ).toBe(false)
})

it('preserves edited JSON on background refresh and failed stale save until explicit reload', async () => {
  let generation = 3
  const transport = createMockTransport({
    'glyphdata.state': () => database({ generation }),
    'glyphdata.set_overlay': () => {
      throw new ApiCallError('stale_generation', 'Document generation changed')
    },
  })
  setTransport(transport)
  render(<GlyphDatabasePanel {...context()} />)
  const editor = await screen.findByRole('textbox', {
    name: 'Custom glyph data JSON',
  })
  await waitFor(() => expect(editor).toHaveValue('{"version":1,"glyphs":{}}'))
  const overlay = '{"version":1,"glyphs":{"draft":{"script":"latin"}}}'
  fireEvent.change(editor, { target: { value: overlay } })
  act(() => {
    generation = 4
    useDocumentStore.getState().noteGeneration(1, 4)
  })
  await waitFor(() =>
    expect(
      transport.calls.filter(({ method }) => method === 'glyphdata.state'),
    ).toHaveLength(2),
  )
  expect(editor).toHaveValue(overlay)
  await userEvent.click(
    screen.getByRole('button', { name: 'Save custom data' }),
  )
  expect(await screen.findByRole('alert')).toHaveTextContent(
    'Document generation changed',
  )
  expect(editor).toHaveValue(overlay)
  expect(transport.calls).toContainEqual(
    expect.objectContaining({
      method: 'glyphdata.set_overlay',
      params: expect.objectContaining({ expected_generation: 3 }),
    }),
  )
  await userEvent.click(
    screen.getByRole('button', { name: 'Reload saved JSON' }),
  )
  await waitFor(() => expect(editor).toHaveValue('{"version":1,"glyphs":{}}'))
})

it('ignores a late response for a previous glyph', async () => {
  let resolveOld!: (state: GlyphDataState) => void
  const transport = createMockTransport({
    'glyphdata.state': ({ gid }) =>
      gid === 42
        ? new Promise<GlyphDataState>((resolve) => {
            resolveOld = resolve
          })
        : database({
            custom: false,
            info: { ...database().info, anchors: ['current-anchor'] },
          }),
  })
  setTransport(transport)
  const props = context()
  const { rerender } = render(<GlyphDatabasePanel {...props} />)
  rerender(<GlyphDatabasePanel {...props} gid={43} />)
  expect(await screen.findByText('current-anchor')).toBeVisible()
  await act(async () => resolveOld(database()))
  expect(screen.queryByText('top, bottom')).not.toBeInTheDocument()
})

it('rejects oversized imports before reading their content and keeps the draft', async () => {
  setTransport(createMockTransport({ 'glyphdata.state': () => database() }))
  render(<GlyphDatabasePanel {...context()} />)
  await screen.findByText('top, bottom')
  const file = new File([], 'huge.json', { type: 'application/json' })
  const read = vi.fn(async () => '{}')
  Object.defineProperty(file, 'size', { value: 4 * 1024 * 1024 + 1 })
  Object.defineProperty(file, 'text', { value: read })
  await userEvent.upload(screen.getByLabelText('Import JSON file'), file)
  expect(await screen.findByRole('alert')).toHaveTextContent('4 MiB')
  expect(read).not.toHaveBeenCalled()
  expect(
    screen.getByRole('textbox', { name: 'Custom glyph data JSON' }),
  ).toHaveValue('{"version":1,"glyphs":{}}')
})

it('shows a failed glyph update without reloading the outline', async () => {
  setTransport(
    createMockTransport({
      'glyphdata.state': () => database(),
      'glyphdata.update': () => {
        throw new Error('Update failed')
      },
    }),
  )
  const props = context()
  render(<GlyphDatabasePanel {...props} />)
  await screen.findByText('top, bottom')
  await userEvent.click(
    screen.getByRole('button', { name: 'Update Glyph Info' }),
  )
  expect(await screen.findByRole('alert')).toHaveTextContent('Update failed')
  expect(props.reload).not.toHaveBeenCalled()
})

it('keeps a new glyph editable while a previous import completes', async () => {
  setTransport(createMockTransport({ 'glyphdata.state': () => database() }))
  const props = context()
  const { rerender } = render(<GlyphDatabasePanel {...props} />)
  await screen.findByText('top, bottom')
  let finish!: (text: string) => void
  const file = new File([], 'old.json', { type: 'application/json' })
  Object.defineProperty(file, 'text', {
    value: () =>
      new Promise<string>((resolve) => {
        finish = resolve
      }),
  })
  await userEvent.upload(screen.getByLabelText('Import JSON file'), file)
  rerender(<GlyphDatabasePanel {...props} gid={43} />)
  await waitFor(() =>
    expect(
      screen.getByRole('button', { name: 'Update Glyph Info' }),
    ).toBeEnabled(),
  )
  rerender(<GlyphDatabasePanel {...props} />)
  await waitFor(() =>
    expect(
      screen.getByRole('button', { name: 'Update Glyph Info' }),
    ).toBeEnabled(),
  )
  await act(async () => finish('{"old":true}'))
  expect(
    screen.getByRole('textbox', { name: 'Custom glyph data JSON' }),
  ).toHaveValue('{"version":1,"glyphs":{}}')
})
