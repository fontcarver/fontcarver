import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { setTransport, type AddGlyphsParams } from '../api'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { cellFor } from '../test/glyphFixtures'
import { createMockTransport, makeSummary } from '../test/mockTransport'

import { AddGlyphsDialog } from './AddGlyphsDialog'

/** Fakes the kernel's recipe grammar closely enough for the dialog's contract. */
function resolve(recipe: string) {
  const names = recipe
    .split(/[\s,;]+/)
    .filter((token) => token.length > 0)
    .flatMap((token) => {
      const range = /^([A-Z])-([A-Z])$/.exec(token)
      if (!range) return [token]
      const [, from = 'A', to = 'A'] = range
      const out: string[] = []
      for (let code = from.charCodeAt(0); code <= to.charCodeAt(0); code += 1)
        out.push(String.fromCharCode(code))
      return out
    })
  return {
    added: names
      .filter((name) => name !== 'a')
      .map((name, index) => cellFor(index, { name })),
    skipped: names.filter((name) => name === 'a'),
  }
}

function setup() {
  const transport = createMockTransport({
    'glyphs.add': ({ recipe }) => resolve(recipe),
    'doc.summary': () => makeSummary({ doc: 1 }),
  })
  setTransport(transport)
  useSessionStore.setState({ docs: [makeSummary({ doc: 1 })] })
  useUiStore.getState().setDialog({ kind: 'addGlyphs', doc: 1 })
  render(<AddGlyphsDialog doc={1} />)
  return transport
}

const addCalls = (transport: ReturnType<typeof setup>) =>
  transport.calls.filter((entry) => entry.method === 'glyphs.add')

beforeEach(() => {
  localStorage.clear()
  useUiStore.setState(initialUiState())
  useSessionStore.setState(initialSessionState())
})

afterEach(() => {
  setTransport(undefined)
})

describe('AddGlyphsDialog', () => {
  it('previews a recipe through a dry run and lists the names', async () => {
    const user = userEvent.setup()
    const transport = setup()

    await user.type(screen.getByLabelText('Recipe'), 'A-C a')

    await waitFor(() =>
      expect(
        addCalls(transport).some(
          (entry) => (entry.params as AddGlyphsParams).recipe === 'A-C a',
        ),
      ).toBe(true),
    )
    const dryRun = addCalls(transport).find(
      (entry) => (entry.params as AddGlyphsParams).recipe === 'A-C a',
    )
    expect(dryRun?.params).toEqual({
      doc: 1,
      recipe: 'A-C a',
      master: null,
      dry_run: true,
    })
    const list = await screen.findByRole('list', { name: 'Glyphs to add' })
    expect(within(list).getAllByRole('listitem')).toHaveLength(3)
    expect(screen.getByText('3 new')).toBeInTheDocument()
    expect(screen.getByText('1 skipped')).toBeInTheDocument()
  })

  it('applies the same recipe without the dry-run flag', async () => {
    const user = userEvent.setup()
    const transport = setup()

    await user.type(screen.getByLabelText('Recipe'), 'A-C')
    const add = await screen.findByRole('button', { name: 'Add 3 glyphs' })
    await user.click(add)

    await waitFor(() => expect(useUiStore.getState().dialog).toBeNull())
    const applied = addCalls(transport).find(
      (entry) => (entry.params as AddGlyphsParams).dry_run === false,
    )
    expect(applied?.params).toEqual({
      doc: 1,
      recipe: 'A-C',
      master: null,
      dry_run: false,
    })
  })

  it('counts each character set against the open document', async () => {
    setup()

    const row = await screen.findByRole('button', { name: /Basic Latin/ })
    await waitFor(() => expect(row).toHaveTextContent(/\d+ new/))
  })

  it('appends the recipe of a chosen character set', async () => {
    const user = userEvent.setup()
    setup()

    await user.click(await screen.findByRole('button', { name: /Latin-1/ }))

    expect(screen.getByLabelText('Recipe')).toHaveValue('U+0020-U+00FF')
  })

  it('keeps Add disabled while the recipe adds nothing', async () => {
    const user = userEvent.setup()
    setup()

    expect(screen.getByRole('button', { name: /^Add/ })).toBeDisabled()
    await user.type(screen.getByLabelText('Recipe'), 'a')
    await waitFor(() =>
      expect(screen.getByRole('button', { name: /^Add/ })).toBeDisabled(),
    )
  })
})
