import { EditorView } from '@codemirror/view'
import { act, fireEvent, render, screen, within } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { setTransport } from '../../api'
import { cellFor } from '../../test/glyphFixtures'
import { createMockTransport } from '../../test/mockTransport'
import { FeatureDraftSession } from './FeatureDraftSession'
import { FeatureSourceEditor } from './FeatureSourceEditor'
import { sourceItems } from './featureSources'
import { featureSettings, featureState } from './featureTestFixtures'

afterEach(() => setTransport(undefined))

describe('FeatureSourceEditor class preview', () => {
  it('opens and closes the preview by keyboard without creating a source edit', async () => {
    const source = featureState().active_source
    source.prefixes[0]!.code = 'sub @Upper by A;'
    source.classes = [{ id: 'c1', name: 'Upper', code: 'A', automatic: false }]
    const session = new FeatureDraftSession(1)
    const change = vi.spyOn(session, 'change')
    setTransport(
      createMockTransport({
        'glyphs.search': () => [cellFor(1, { name: 'A' })],
        'glyphs.cell': () => cellFor(1, { name: 'A' }),
      }),
    )
    render(
      <FeatureSourceEditor
        source={source}
        item={sourceItems(source)[0]!}
        settings={featureSettings()}
        automaticEntries={[]}
        generated={undefined}
        generatedSaved={false}
        readOnly={true}
        glyphNames={['A']}
        loadGlyphNames={async () => ['A']}
        diagnostics={[]}
        focusRange={null}
        session={session}
        master="m1"
        generation={1}
        onRemove={vi.fn()}
      />,
    )
    const target = screen.getByRole('textbox', { name: 'Feature source code' })
    const view = EditorView.findFromDOM(target)!
    act(() => view.dispatch({ selection: { anchor: 8 } }))
    fireEvent.keyDown(target, { key: 'Enter', altKey: true })
    const preview = await screen.findByRole('region', { name: 'Class @Upper' })
    expect(
      await within(preview).findByRole('img', { name: 'Outline of A' }),
    ).toBeVisible()
    fireEvent.click(within(preview).getByRole('button', { name: /Close/ }))
    expect(
      screen.queryByRole('region', { name: 'Class @Upper' }),
    ).not.toBeInTheDocument()
    expect(change).not.toHaveBeenCalled()
    expect(view.state.doc.toString()).toBe('sub @Upper by A;')
  })
})
