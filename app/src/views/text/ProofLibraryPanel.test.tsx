import { createRef } from 'react'
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { ProofLibraryPanel } from './ProofLibraryPanel'
import { setTransport } from '../../api'
import { createMockTransport, noChange } from '../../test/mockTransport'
import { makeFontInfoView } from '../../test/fontFixtures'
import { writeProofLibrary } from './proofLibrary'
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
beforeEach(() => useDocumentStore.setState(initialDocumentState()))
afterEach(() => setTransport(undefined))

it('steps saved strings from the text field and restores their shaping context', async () => {
  const info = makeFontInfoView()
  info.info.user_data = writeProofLibrary(null, entries)
  setTransport(createMockTransport({ 'doc.font_info': () => info }))
  const inputRef = createRef<HTMLInputElement>()
  const onSelect = vi.fn()
  render(
    <>
      <input ref={inputRef} aria-label="Test text" />
      <ProofLibraryPanel
        context={context}
        title="Test proof"
        inputRef={inputRef}
        onSelect={onSelect}
      />
    </>,
  )
  fireEvent.click(screen.getByText('Proof library'))
  await screen.findByRole('option', { name: 'Pairs · AV pair' })
  fireEvent.keyDown(inputRef.current!, { key: 'ArrowDown' })
  expect(onSelect).toHaveBeenLastCalledWith(entries[0])
  fireEvent.keyDown(inputRef.current!, { key: 'ArrowUp' })
  expect(onSelect).toHaveBeenLastCalledWith(entries[2])
  fireEvent.change(screen.getByRole('combobox', { name: 'All groups' }), {
    target: { value: 'Pairs' },
  })
  fireEvent.keyDown(inputRef.current!, { key: 'ArrowDown' })
  expect(onSelect).toHaveBeenLastCalledWith(entries[0])
  fireEvent.keyDown(inputRef.current!, { key: 'ArrowDown', shiftKey: true })
  expect(onSelect).toHaveBeenCalledTimes(3)
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
