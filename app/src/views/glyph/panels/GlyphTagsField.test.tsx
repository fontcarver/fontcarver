import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import { i18n } from '../../../i18n'
import {
  initialDocumentState,
  newDocView,
  useDocumentStore,
} from '../../../stores/documentStore'
import { GlyphTagsField } from './GlyphTagsField'
import { completeTag, parseTags, tagSuggestions } from './glyphTags'

beforeEach(async () => {
  await i18n.changeLanguage('en')
  useDocumentStore.setState({
    ...initialDocumentState(),
    byDoc: {
      1: {
        ...newDocView(),
        sidebarGeneration: 0,
        sidebar: {
          total: 3,
          unencoded: 0,
          categories: [],
          scripts: [],
          blocks: [],
          color_labels: [],
          tags: ['review', 'ready', 'Latin', 'review'],
        },
      },
    },
  })
})

describe('glyph tags', () => {
  it('normalizes free-text tags and bounds distinct completions', () => {
    expect(parseTags(' review, review, custom tag, , ')).toEqual([
      'review',
      'custom tag',
    ])
    expect(completeTag('Latin, re', 'review')).toEqual(['Latin', 'review'])
    expect(
      tagSuggestions('Latin, RE', ['Latin', 'ready', 'review', 'review']),
    ).toEqual(['ready', 'review'])
    expect(
      tagSuggestions(
        '',
        Array.from({ length: 40 }, (_, index) => String(index)),
      ),
    ).toHaveLength(8)
  })
  it('selects completion using arrow keys and Enter, and permits a new free-text tag', async () => {
    const user = userEvent.setup()
    const commit = vi.fn()
    render(<GlyphTagsField doc={1} tags={['Latin']} onCommit={commit} />)
    const input = screen.getByRole('combobox', { name: 'Tags' })
    await user.click(input)
    await user.clear(input)
    await user.type(input, 'Latin, re{ArrowDown}{Enter}')
    expect(commit).toHaveBeenLastCalledWith(['Latin', 'ready'])
    expect(input).toHaveAttribute('aria-expanded', 'false')
    await user.clear(input)
    await user.type(input, 'own tag{Enter}')
    expect(commit).toHaveBeenLastCalledWith(['own tag'])
  })
  it('cancels on Escape and follows external undo changes', async () => {
    const user = userEvent.setup()
    const commit = vi.fn()
    const { rerender } = render(
      <GlyphTagsField doc={1} tags={['Latin']} onCommit={commit} />,
    )
    const input = screen.getByRole('combobox', { name: 'Tags' })
    await user.click(input)
    await user.type(input, ', changed{Escape}')
    expect(commit).not.toHaveBeenCalled()
    expect(input).toHaveValue('Latin')
    rerender(<GlyphTagsField doc={1} tags={['review']} onCommit={commit} />)
    expect(input).toHaveValue('review')
  })
})
