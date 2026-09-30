import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it } from 'vitest'
import {
  emptyFilter,
  initialDocumentState,
  useDocumentStore,
} from '../../stores/documentStore'
import { initialUiState, useUiStore } from '../../stores/uiStore'
import { LanguageFilters } from './LanguageFilters'

beforeEach(() => {
  useDocumentStore.setState(initialDocumentState())
  useDocumentStore.getState().ensure(1)
  useUiStore.setState(initialUiState())
})

describe('language repertoire filters', () => {
  it('counts distinct scalar assignments and previews only missing characters', async () => {
    const user = userEvent.setup()
    render(
      <LanguageFilters
        doc={1}
        filter={emptyFilter()}
        unicodes={[65, 65, 97]}
      />,
    )
    expect(
      screen.getByRole('button', { name: 'English 2 / 52' }),
    ).toBeInTheDocument()
    await user.click(
      screen.getByRole('button', {
        name: 'Preview 50 missing characters for English',
      }),
    )
    const dialog = useUiStore.getState().dialog
    expect(dialog?.kind).toBe('addGlyphs')
    if (dialog?.kind !== 'addGlyphs')
      throw new Error('Expected add glyphs dialog')
    expect(dialog.recipe?.split(' ')).toHaveLength(50)
    expect(dialog.recipe).not.toContain('U+0041')
    expect(dialog.recipe).not.toContain('U+0061')
    expect(dialog.recipe).toContain('U+0042')
  })

  it('selects only repertoire letters and clears incompatible earlier filters', async () => {
    const user = userEvent.setup()
    const filter = { ...emptyFilter(), query: 'A', unencoded_only: true }
    render(<LanguageFilters doc={1} filter={filter} unicodes={[]} />)
    await user.click(screen.getByRole('button', { name: 'English 0 / 52' }))
    expect(useDocumentStore.getState().byDoc[1].filter).toMatchObject({
      query: null,
      unencoded_only: false,
      unicode_ranges: [
        [65, 90],
        [97, 122],
      ],
    })
  })

  it('disables generation when every letter is encoded', () => {
    const letters = [
      ...Array.from({ length: 26 }, (_, i) => i + 65),
      ...Array.from({ length: 26 }, (_, i) => i + 97),
    ]
    render(
      <LanguageFilters doc={1} filter={emptyFilter()} unicodes={letters} />,
    )
    expect(
      screen.getByRole('button', {
        name: 'Preview 0 missing characters for English',
      }),
    ).toBeDisabled()
  })
})
