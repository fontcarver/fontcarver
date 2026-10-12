import { fireEvent, render, screen } from '@testing-library/react'
import { beforeEach, expect, it, vi } from 'vitest'

import type { GlyphFilter } from '../../api'
import { i18n } from '../../i18n'
import {
  emptyFilter,
  initialDocumentState,
  isFiltered,
  useDocumentStore,
} from '../../stores/documentStore'

import { FilterChips, filterChips } from './FilterChips'

beforeEach(() => {
  useDocumentStore.setState(initialDocumentState())
  useDocumentStore.getState().ensure(1)
})

function ActiveChips() {
  const filter = useDocumentStore((state) => state.byDoc[1]?.filter)
  return filter ? <FilterChips doc={1} filter={filter} /> : null
}

function removeChip(key: string, options: Record<string, unknown> = {}) {
  fireEvent.click(
    screen.getByRole('button', {
      name: i18n.t('fontView.removeFilter', { name: i18n.t(key, options) }),
    }),
  )
}

it('recognizes smart expressions and explicit empty name sets as active filters', () => {
  expect(
    isFiltered({ ...emptyFilter(), smart_expression: 'name == "A"' }),
  ).toBe(true)
  expect(isFiltered({ ...emptyFilter(), smart_expression: ' ' })).toBe(false)
  expect(isFiltered({ ...emptyFilter(), glyph_names: [] })).toBe(true)
  expect(isFiltered({ ...emptyFilter(), glyph_names: ['A'] })).toBe(true)
  expect(isFiltered({ ...emptyFilter(), glyph_names: null })).toBe(false)
})

it('does not treat unused query options or an empty query as active filters', () => {
  const filter: GlyphFilter = {
    ...emptyFilter(),
    query: '',
    query_scope: 'Note',
    case_sensitive: true,
    regex: true,
  }
  expect(isFiltered(filter)).toBe(false)
  expect(filterChips(filter, (key) => key)).toEqual([])
})

it('counts selected names including an explicit set matching no glyphs', () => {
  const label = vi.fn((key: string) => key)
  const chips = filterChips({ ...emptyFilter(), glyph_names: [] }, label)
  expect(chips).toEqual([
    {
      key: 'glyph_names',
      label: 'smartFilters.namesChip',
      patch: { glyph_names: null },
    },
  ])
  expect(label).toHaveBeenCalledWith('smartFilters.namesChip', { count: 0 })
})

it('removes smart and name filters separately while retaining the query', () => {
  useDocumentStore.getState().setFilter(1, {
    query: 'A',
    smart_expression: 'width > 500',
    glyph_names: ['A', 'A.alt'],
  })
  useDocumentStore.getState().setMatchCount(1, 2)
  render(<ActiveChips />)
  expect(screen.getByTitle('width > 500')).toHaveAccessibleName(
    i18n.t('fontView.removeFilter', { name: i18n.t('smartFilters.chip') }),
  )
  removeChip('smartFilters.chip')
  expect(useDocumentStore.getState().byDoc[1]?.filter).toMatchObject({
    query: 'A',
    smart_expression: null,
    glyph_names: ['A', 'A.alt'],
  })
  expect(useDocumentStore.getState().byDoc[1]?.matchCount).toBeNull()
  removeChip('smartFilters.namesChip', { count: 2 })
  expect(useDocumentStore.getState().byDoc[1]?.filter).toMatchObject({
    query: 'A',
    glyph_names: null,
  })
})

it('removes individual query options without losing the query', () => {
  useDocumentStore.getState().setFilter(1, {
    query: '^A',
    query_scope: 'Name',
    case_sensitive: true,
    regex: true,
  })
  render(<ActiveChips />)
  removeChip('glyphSearch.scope.Name')
  removeChip('glyphSearch.caseSensitive')
  removeChip('glyphSearch.regex')
  expect(useDocumentStore.getState().byDoc[1]?.filter).toMatchObject({
    query: '^A',
    query_scope: 'All',
    case_sensitive: false,
    regex: false,
  })
})

it('clears every query option and smart condition with the other filters', () => {
  useDocumentStore.getState().setFilter(1, {
    query: 'A',
    query_scope: 'Note',
    case_sensitive: true,
    regex: true,
    smart_expression: 'width > 500',
    glyph_names: [],
    categories: ['Letter'],
  })
  render(<ActiveChips />)
  fireEvent.click(screen.getByRole('button', { name: 'Clear filters' }))
  expect(useDocumentStore.getState().byDoc[1]?.filter).toMatchObject({
    query: null,
    query_scope: 'All',
    case_sensitive: false,
    regex: false,
    smart_expression: null,
    glyph_names: null,
    categories: [],
  })
  expect(screen.queryByRole('button')).not.toBeInTheDocument()
})
