import { render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { initialEditorState, useEditorStore } from '../../stores/editorStore'
import {
  makeCell,
  makeGlyph,
  makeGlyphDetail,
  makeMaster,
} from '../../test/fontFixtures'

import { GlyphInspector } from './GlyphInspector'
import { buildScene } from './glyphScene'

beforeEach(() => {
  useEditorStore.setState(initialEditorState())
})

afterEach(() => {
  useEditorStore.setState(initialEditorState())
})

describe('GlyphInspector', () => {
  it('shows the classification derived by glyphs.detail', () => {
    const glyph = makeGlyph({
      category: 'Mark',
      sub_category: 'Nonspacing',
      script: 'Zinh',
    })
    const detail = makeGlyphDetail(glyph)
    const cell = makeCell({
      category: 'Mark',
      sub_category: 'Nonspacing',
      script: 'Zinh',
    })
    const scene = buildScene({
      detail,
      cell,
      master: makeMaster(),
      masterId: 'm1',
      unitsPerEm: 2048,
    })
    useEditorStore.getState().setActiveGlyph({
      doc: 1,
      gid: cell.gid,
      glyph,
      scene,
      cell,
    })

    render(<GlyphInspector />)

    expect(screen.getByText('Mark')).toBeInTheDocument()
    expect(screen.getByText('Nonspacing')).toBeInTheDocument()
    expect(screen.getByText('Zinh')).toBeInTheDocument()
  })
})
