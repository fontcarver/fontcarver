import { createRef, useRef, useState } from 'react'
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { ProofLibraryPanel } from './ProofLibraryPanel'
import { setTransport } from '../../api'
import { createMockTransport, noChange } from '../../test/mockTransport'
import { makeFontInfoView } from '../../test/fontFixtures'
import { writeProofLibrary, type ProofEntry } from './proofLibrary'
import { BUILTIN_PROOF_SAMPLES } from './proofSamples'
import {
  initialDocumentState,
  useDocumentStore,
} from '../../stores/documentStore'

const context = {
  doc: 1,
  generation: 0,
  unitsPerEm: 1000,
  master: 'm1',
  text: 'AV',
  features: [],
  direction: 'ltr',
  script: 'Latn',
  language: 'en',
}
const entries = [
  {
    id: 'av',
    name: 'AV pair',
    text: 'AV',
    group: 'Pairs',
    script: 'Latn',
    language: 'en',
    direction: 'ltr' as const,
  },
  {
    id: 'to',
    name: 'To pair',
    text: 'To',
    group: 'Pairs',
    script: 'Latn',
    language: 'en',
    direction: 'ltr' as const,
  },
  {
    id: 'ar',
    name: 'Arabic',
    text: 'سلام',
    group: 'Phrases',
    script: 'Arab',
    language: 'ar',
    direction: 'rtl' as const,
  },
]

/** Holds the selected proof the way the text tab does. */
function Harness({
  onSelect,
  withSurface = false,
}: {
  onSelect: (entry: ProofEntry) => void
  withSurface?: boolean
}) {
  const inputRef = useRef<HTMLInputElement>(null)
  const stepRef = useRef<((direction: -1 | 1) => void) | null>(null)
  const [selectedId, setSelectedId] = useState<string | null>(null)
  return (
    <>
      <input ref={inputRef} aria-label="Test text" />
      {withSurface && (
        <button type="button" onClick={() => stepRef.current?.(1)}>
          Surface step
        </button>
      )}
      <ProofLibraryPanel
        context={context}
        title="Test proof"
        inputRef={inputRef}
        selectedId={selectedId}
        stepRef={stepRef}
        onSelect={(entry) => {
          setSelectedId(entry.id)
          onSelect(entry)
        }}
      />
    </>
  )
}

function savedLibrary() {
  const info = makeFontInfoView()
  info.info.user_data = writeProofLibrary(null, entries)
  return info
}

beforeEach(() => useDocumentStore.setState(initialDocumentState()))
afterEach(() => setTransport(undefined))

it('steps saved strings from the text field and restores their shaping context', async () => {
  setTransport(createMockTransport({ 'doc.font_info': () => savedLibrary() }))
  const onSelect = vi.fn()
  render(<Harness onSelect={onSelect} />)
  const input = screen.getByRole('textbox', { name: 'Test text' })
  fireEvent.click(screen.getByText('Proof library'))
  await screen.findByRole('option', { name: 'Pairs · AV pair' })
  fireEvent.keyDown(input, { key: 'ArrowDown' })
  expect(onSelect).toHaveBeenLastCalledWith(entries[0])
  fireEvent.keyDown(input, { key: 'ArrowDown' })
  expect(onSelect).toHaveBeenLastCalledWith(entries[1])
  fireEvent.keyDown(input, { key: 'ArrowUp' })
  expect(onSelect).toHaveBeenLastCalledWith(entries[0])
  fireEvent.keyDown(input, { key: 'ArrowUp' })
  expect(onSelect).toHaveBeenLastCalledWith(entries[2])
  fireEvent.change(screen.getByRole('combobox', { name: 'All groups' }), {
    target: { value: 'Pairs' },
  })
  fireEvent.keyDown(input, { key: 'ArrowDown' })
  expect(onSelect).toHaveBeenLastCalledWith(entries[0])
  fireEvent.keyDown(input, { key: 'ArrowDown', shiftKey: true })
  expect(onSelect).toHaveBeenCalledTimes(5)
})

it('steps from the surrounding view while a glyph is being spaced', async () => {
  setTransport(createMockTransport({ 'doc.font_info': () => savedLibrary() }))
  const onSelect = vi.fn()
  render(<Harness onSelect={onSelect} withSurface />)
  fireEvent.click(screen.getByText('Proof library'))
  await screen.findByRole('option', { name: 'Pairs · AV pair' })
  const surface = screen.getByRole('button', { name: 'Surface step' })
  fireEvent.click(surface)
  expect(onSelect).toHaveBeenLastCalledWith(entries[0])
  fireEvent.click(surface)
  expect(onSelect).toHaveBeenLastCalledWith(entries[1])
})

it('filters by script and keeps stepping inside the visible list', async () => {
  setTransport(createMockTransport({ 'doc.font_info': () => savedLibrary() }))
  const onSelect = vi.fn()
  render(<Harness onSelect={onSelect} />)
  fireEvent.click(screen.getByText('Proof library'))
  await screen.findByRole('option', { name: 'Pairs · AV pair' })
  fireEvent.change(screen.getByRole('combobox', { name: 'Filter by script' }), {
    target: { value: 'Arab' },
  })
  fireEvent.keyDown(screen.getByRole('textbox', { name: 'Test text' }), {
    key: 'ArrowDown',
  })
  expect(onSelect).toHaveBeenLastCalledWith(entries[2])
  fireEvent.keyDown(screen.getByRole('textbox', { name: 'Test text' }), {
    key: 'ArrowDown',
  })
  expect(onSelect).toHaveBeenLastCalledWith(entries[2])
})

it('saves the current text and metadata through the undoable project command', async () => {
  const info = makeFontInfoView()
  const transport = createMockTransport({
    'doc.font_info': () => info,
    'doc.apply': () => noChange(),
  })
  setTransport(transport)
  render(
    <ProofLibraryPanel
      context={context}
      title="Test proof"
      inputRef={createRef()}
      onSelect={vi.fn()}
    />,
  )
  fireEvent.click(screen.getByText('Proof library'))
  await waitFor(() =>
    expect(
      screen.getByRole('button', { name: 'Save current string' }),
    ).toBeEnabled(),
  )
  fireEvent.change(screen.getByRole('textbox', { name: 'Proof name' }), {
    target: { value: 'AV saved' },
  })
  fireEvent.click(screen.getByRole('button', { name: 'Save current string' }))
  await waitFor(() =>
    expect(transport.calls.some((call) => call.method === 'doc.apply')).toBe(
      true,
    ),
  )
  expect(
    JSON.stringify(
      transport.calls.find((call) => call.method === 'doc.apply')?.params,
    ),
  ).toContain('com.fontcarver.proofLibrary')
  expect(
    JSON.stringify(
      transport.calls.find((call) => call.method === 'doc.apply')?.params,
    ),
  ).toContain('AV saved')
})

it('adopts the built-in grouped samples as one undoable project command', async () => {
  const info = makeFontInfoView()
  const transport = createMockTransport({
    'doc.font_info': () => info,
    'doc.apply': () => noChange(),
  })
  setTransport(transport)
  render(
    <ProofLibraryPanel
      context={context}
      title="Test proof"
      inputRef={createRef()}
      onSelect={vi.fn()}
    />,
  )
  fireEvent.click(screen.getByText('Proof library'))
  await waitFor(() =>
    expect(
      screen.getByRole('button', { name: 'Add built-in samples' }),
    ).toBeEnabled(),
  )
  fireEvent.click(screen.getByRole('button', { name: 'Add built-in samples' }))
  await waitFor(() =>
    expect(transport.calls.some((call) => call.method === 'doc.apply')).toBe(
      true,
    ),
  )
  const applied = transport.calls.filter((call) => call.method === 'doc.apply')
  expect(applied).toHaveLength(1)
  const payload = JSON.stringify(applied[0]?.params)
  for (const sample of BUILTIN_PROOF_SAMPLES)
    expect(payload).toContain(sample.id)
  expect(payload).toContain('Latin pangram')
  expect(payload).toContain('Han vertical sample')
})

it('merges an imported list and reports a file it cannot read', async () => {
  const info = savedLibrary()
  const transport = createMockTransport({
    'doc.font_info': () => info,
    'doc.apply': () => noChange(),
  })
  setTransport(transport)
  render(
    <ProofLibraryPanel
      context={context}
      title="Test proof"
      inputRef={createRef()}
      onSelect={vi.fn()}
    />,
  )
  fireEvent.click(screen.getByText('Proof library'))
  await screen.findByRole('option', { name: 'Pairs · AV pair' })
  const picker = document.querySelector(
    'input[type="file"]',
  ) as HTMLInputElement
  const good = new File(
    ['Pairs\tAV pair\tLatn\ten\tltr\tAV corrected\n'],
    'proofs.txt',
    { type: 'text/plain' },
  )
  fireEvent.change(picker, { target: { files: [good] } })
  await waitFor(() =>
    expect(transport.calls.some((call) => call.method === 'doc.apply')).toBe(
      true,
    ),
  )
  const payload = JSON.stringify(
    transport.calls.find((call) => call.method === 'doc.apply')?.params,
  )
  expect(payload).toContain('AV corrected')
  // The entry it names keeps its identifier, and the rest of the library stays.
  expect(payload).toContain('"id":"av"')
  expect(payload).toContain('To pair')

  const bad = new File(['not a proof list'], 'bad.txt', { type: 'text/plain' })
  fireEvent.change(picker, { target: { files: [bad] } })
  const alert = await screen.findByRole('alert')
  expect(alert).toHaveTextContent('Nothing was imported.')
  expect(alert).toHaveTextContent('invalid or excessive data')
})
