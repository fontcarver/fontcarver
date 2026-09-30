import { act, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'

import {
  initialDocumentState,
  useDocumentStore,
} from '../../stores/documentStore'

import { GlyphSearch, SEARCH_DEBOUNCE_MS } from './GlyphSearch'

beforeEach(() => {
  useDocumentStore.setState(initialDocumentState())
  useDocumentStore.getState().ensure(1)
  vi.useFakeTimers()
})

afterEach(() => {
  vi.useRealTimers()
})

it('debounces literal queries and displays externally cleared queries', () => {
  const { rerender } = render(<GlyphSearch doc={1} value={null} />)
  const field = screen.getByRole('searchbox', { name: 'Search glyphs' })
  fireEvent.change(field, { target: { value: '  U+0041  ' } })
  expect(useDocumentStore.getState().byDoc[1]?.filter.query).toBeNull()
  act(() => vi.advanceTimersByTime(SEARCH_DEBOUNCE_MS))
  expect(useDocumentStore.getState().byDoc[1]?.filter.query).toBe('U+0041')
  rerender(<GlyphSearch doc={1} value="U+0041" />)
  expect(field).toHaveValue('U+0041')
  rerender(<GlyphSearch doc={1} value={null} />)
  expect(field).toHaveValue('')
})

it('updates scope, case and regex options without clearing other filters', () => {
  useDocumentStore.getState().setFilter(1, { categories: ['Letter'] })
  render(<GlyphSearch doc={1} value={null} />)
  fireEvent.change(screen.getByRole('combobox', { name: 'Search scope' }), {
    target: { value: 'Note' },
  })
  fireEvent.click(screen.getByRole('button', { name: 'Match case' }))
  fireEvent.click(
    screen.getByRole('button', { name: 'Use regular expression' }),
  )
  expect(useDocumentStore.getState().byDoc[1]?.filter).toMatchObject({
    categories: ['Letter'],
    query_scope: 'Note',
    case_sensitive: true,
    regex: true,
  })
})

it('uses controlled option callbacks and reflects their selected values', () => {
  const onQueryScopeChange = vi.fn()
  const onCaseSensitiveChange = vi.fn()
  const onRegexChange = vi.fn()
  render(
    <GlyphSearch
      doc={1}
      value={null}
      queryScope="Unicode"
      caseSensitive
      regex
      onQueryScopeChange={onQueryScopeChange}
      onCaseSensitiveChange={onCaseSensitiveChange}
      onRegexChange={onRegexChange}
    />,
  )
  const scope = screen.getByRole('combobox', { name: 'Search scope' })
  expect(scope).toHaveValue('Unicode')
  const matchCase = screen.getByRole('button', { name: 'Match case' })
  const regex = screen.getByRole('button', { name: 'Use regular expression' })
  expect(matchCase).toHaveAttribute('aria-pressed', 'true')
  expect(regex).toHaveAttribute('aria-pressed', 'true')
  fireEvent.change(scope, { target: { value: 'Name' } })
  fireEvent.click(matchCase)
  fireEvent.click(regex)
  expect(onQueryScopeChange).toHaveBeenCalledWith('Name')
  expect(onCaseSensitiveChange).toHaveBeenCalledWith(false)
  expect(onRegexChange).toHaveBeenCalledWith(false)
  expect(useDocumentStore.getState().byDoc[1]?.filter).not.toHaveProperty(
    'query_scope',
    'Name',
  )
})

it('preserves significant whitespace in a regular expression', () => {
  render(<GlyphSearch doc={1} value={null} regex />)
  fireEvent.change(screen.getByRole('searchbox'), {
    target: { value: ' ^A ' },
  })
  act(() => vi.advanceTimersByTime(SEARCH_DEBOUNCE_MS))
  expect(useDocumentStore.getState().byDoc[1]?.filter.query).toBe(' ^A ')
})

it('cancels an uncommitted query when the search is removed', () => {
  const { unmount } = render(<GlyphSearch doc={1} value={null} />)
  fireEvent.change(screen.getByRole('searchbox'), {
    target: { value: 'uncommitted' },
  })
  unmount()
  act(() => vi.advanceTimersByTime(SEARCH_DEBOUNCE_MS))
  expect(useDocumentStore.getState().byDoc[1]?.filter.query).toBeNull()
})
