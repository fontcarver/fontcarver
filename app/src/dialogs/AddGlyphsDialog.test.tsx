import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { setTransport, type AddGlyphsParams } from '../api'
import { i18n } from '../i18n'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { cellFor } from '../test/glyphFixtures'
import { createMockTransport, makeSummary } from '../test/mockTransport'
import { ApiCallError } from '../transport'

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
  const skipped = names.filter(
    (name) => name === 'a' || !/^[A-Za-z_.][A-Za-z0-9_.]*$/.test(name),
  )
  return {
    added: names
      .filter((name) => !skipped.includes(name))
      .map((name, index) => cellFor(index, { name })),
    skipped,
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

/** The most recent error toast, if the dialog raised one. */
const lastError = () =>
  useUiStore
    .getState()
    .toasts.filter((toast) => toast.kind === 'error')
    .at(-1)

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
    expect(screen.getByText('1 already present')).toBeInTheDocument()
  })

  it('says why each item was skipped', async () => {
    const user = userEvent.setup()
    setup()

    await user.type(
      screen.getByLabelText('Recipe'),
      'a 12three too-large-marker',
    )

    expect(await screen.findByText('1 already present')).toBeInTheDocument()
    expect(screen.getByText('2 not understood')).toBeInTheDocument()
  })

  it('stops counting a character set the kernel could not resolve', async () => {
    const transport = createMockTransport({
      'glyphs.add': ({ recipe }) => {
        if (recipe.startsWith('@charset:'))
          throw new Error('unknown character set')
        return resolve(recipe)
      },
    })
    setTransport(transport)
    useSessionStore.setState({ docs: [makeSummary({ doc: 1 })] })
    render(<AddGlyphsDialog doc={1} />)

    const row = await screen.findByRole('button', { name: /GB 2312/ })
    await waitFor(() => expect(row).toHaveTextContent('—'))
    expect(row).not.toHaveTextContent('counting')
  })

  it('confirms with Enter from outside the recipe field', async () => {
    const user = userEvent.setup()
    const transport = setup()

    await user.type(screen.getByLabelText('Recipe'), 'A-C')
    await waitFor(() =>
      expect(
        screen.getByRole('button', { name: 'Add 3 glyphs' }),
      ).toBeEnabled(),
    )
    // The recipe field and the buttons own Enter themselves; the preview list
    // is the one focusable control in the dialog that does not.
    screen.getByRole('list', { name: 'Glyphs to add' }).focus()
    await user.keyboard('{Enter}')

    await waitFor(() =>
      expect(
        addCalls(transport).some(
          (entry) => (entry.params as AddGlyphsParams).dry_run === false,
        ),
      ).toBe(true),
    )
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

  it('reports a rejected dry run by its kernel error code', async () => {
    const user = userEvent.setup()
    const raw = 'unexpected token `bogus`'
    const transport = createMockTransport({
      'glyphs.add': ({ recipe }) => {
        if (recipe === 'bogus') throw new ApiCallError('invalid_params', raw)
        return resolve(recipe)
      },
    })
    setTransport(transport)
    useSessionStore.setState({ docs: [makeSummary({ doc: 1 })] })
    useUiStore.getState().setDialog({ kind: 'addGlyphs', doc: 1 })
    render(<AddGlyphsDialog doc={1} />)

    await user.type(screen.getByLabelText('Recipe'), 'bogus')

    // The shared reporter's toast: the summary its code maps to, and the
    // kernel's own wording kept under "Details" rather than shown raw.
    await waitFor(() => expect(lastError()).toBeDefined())
    expect(lastError()?.message).toBe(i18n.t('errors.invalid_params'))
    expect(lastError()?.message).not.toBe(raw)
    expect(lastError()?.details).toBe(raw)
    // The dialog stays open on the recipe that failed.
    expect(useUiStore.getState().dialog).not.toBeNull()
  })

  it('reports a rejected add by its kernel error code', async () => {
    const user = userEvent.setup()
    const raw = 'glyph order is locked'
    const transport = createMockTransport({
      'glyphs.add': ({ recipe, dry_run: dryRun }) => {
        if (!dryRun) throw new ApiCallError('internal', raw)
        return resolve(recipe)
      },
    })
    setTransport(transport)
    useSessionStore.setState({ docs: [makeSummary({ doc: 1 })] })
    useUiStore.getState().setDialog({ kind: 'addGlyphs', doc: 1 })
    render(<AddGlyphsDialog doc={1} />)

    await user.type(screen.getByLabelText('Recipe'), 'A-C')
    await user.click(
      await screen.findByRole('button', { name: 'Add 3 glyphs' }),
    )

    await waitFor(() => expect(lastError()).toBeDefined())
    expect(lastError()?.message).toBe(i18n.t('errors.internal'))
    expect(lastError()?.message).not.toBe(raw)
    expect(lastError()?.details).toBe(raw)
    // A failed add leaves the dialog open and the button usable again.
    expect(useUiStore.getState().dialog).not.toBeNull()
    expect(screen.getByRole('button', { name: 'Add 3 glyphs' })).toBeEnabled()
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
