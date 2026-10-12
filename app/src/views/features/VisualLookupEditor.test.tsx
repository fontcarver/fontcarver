import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { setTransport, type FeatureState } from '../../api'
import { flushDocumentEditParticipants } from '../../editor/documentEditParticipants'
import { useDocumentStore } from '../../stores/documentStore'
import { createMockTransport, makeSummary } from '../../test/mockTransport'
import { FeaturesView } from './FeaturesView'
import { featureState, featureValidation } from './featureTestFixtures'

let state: FeatureState
beforeEach(() => {
  state = featureState()
  state.active_source.prefixes = []
  state.active_source.features = [
    {
      id: 'rule1',
      tag: 'salt',
      code: 'sub A by A.alt;',
      automatic: false,
      disabled: false,
      notes: '',
    },
  ]
  useDocumentStore.getState().ensure(1)
  useDocumentStore.getState().setMaster(1, 'm1')
})
afterEach(() => setTransport(undefined))
function setup() {
  const transport = createMockTransport({
    'features.get': () => structuredClone(state),
    'glyphs.list': () => ({ total: 0, items: [] }),
    'features.set_draft': (params) => {
      state = {
        ...state,
        generation: state.generation + 1,
        draft_revision: state.draft_revision + 1,
        has_unapplied_changes: true,
        draft: {
          source: params.source,
          settings: params.settings,
          revision: state.draft_revision + 1,
        },
      }
      return structuredClone(state)
    },
    'features.validate': () => featureValidation(state),
    'features.apply': () => {
      const validation = featureValidation(state)
      state = {
        ...state,
        active_source: state.draft?.source ?? state.active_source,
        draft: null,
        has_unapplied_changes: false,
        generation: state.generation + 1,
        compile_revision: state.compile_revision + 1,
        draft_revision: state.draft_revision + 1,
      }
      return { applied: true, state: structuredClone(state), validation }
    },
  })
  setTransport(transport)
  const view = render(<FeaturesView summary={makeSummary()} />)
  return { transport, view }
}
async function openVisual() {
  fireEvent.click(await screen.findByRole('button', { name: 'Visual editor' }))
  await screen.findByRole('region', { name: 'Visual lookup editor' })
}

describe('Visual lookup editor', () => {
  it('round-trips FEA edits through the draft and checked apply', async () => {
    setup()
    await openVisual()
    expect(screen.getByRole('textbox', { name: 'Input glyphs' })).toHaveValue(
      'A',
    )
    fireEvent.change(screen.getByRole('textbox', { name: 'Output glyphs' }), {
      target: { value: 'A.sc' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Check source' }))
    await screen.findByText('Source compiles in 1 master.')
    expect(state.draft?.source.features[0]?.code).toBe('sub \\A by \\A.sc;')
    expect(state.active_source.features[0]?.code).toBe('sub A by A.alt;')
    fireEvent.click(screen.getByRole('button', { name: 'Compile & preview' }))
    await waitFor(() =>
      expect(state.active_source.features[0]?.code).toBe('sub \\A by \\A.sc;'),
    )
    expect(state.draft).toBeNull()
  })

  it('persists incomplete edits across the document save barrier', async () => {
    setup()
    await openVisual()
    fireEvent.change(screen.getByRole('textbox', { name: 'Output glyphs' }), {
      target: { value: '' },
    })
    await flushDocumentEditParticipants(1)
    expect(state.draft?.source.features[0]?.code).toBe('sub \\A by ;')
    expect(screen.getByRole('textbox', { name: 'Output glyphs' })).toHaveValue(
      '',
    )
    expect(state.active_source.features[0]?.code).toBe('sub A by A.alt;')
  })

  it('keeps incomplete operands editable after switching modes', async () => {
    setup()
    await openVisual()
    fireEvent.change(screen.getByRole('textbox', { name: 'Output glyphs' }), {
      target: { value: '' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Source' }))
    await screen.findByRole('textbox', { name: 'Feature source code' })
    await openVisual()
    expect(screen.getByRole('textbox', { name: 'Output glyphs' })).toHaveValue(
      '',
    )
  })

  it('does not overwrite unsupported syntax and can open it in the source editor', async () => {
    state.active_source.features[0]!.code =
      'script arab; language URD; sub A by A.alt;'
    const { transport } = setup()
    await openVisual()
    expect(
      screen.getByText(/This source uses syntax outside these forms/),
    ).toBeVisible()
    expect(screen.queryByRole('textbox', { name: 'Input glyphs' })).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'Edit feature source' }))
    await screen.findByRole('textbox', { name: 'Feature source code' })
    expect(
      transport.calls.some(({ method }) => method === 'features.set_draft'),
    ).toBe(false)
  })

  it('adds a manually editable lookup without changing generated blocks', async () => {
    state.active_source.features[0]!.automatic = true
    const original = structuredClone(state.active_source.features[0])
    setup()
    await openVisual()
    expect(screen.getByRole('textbox', { name: 'Input glyphs' })).toBeDisabled()
    fireEvent.change(
      screen.getByRole('combobox', { name: 'New lookup type' }),
      { target: { value: 'gsub-ligature' } },
    )
    fireEvent.click(screen.getByRole('button', { name: 'Add lookup' }))
    await flushDocumentEditParticipants(1)
    expect(state.draft?.source.features).toHaveLength(2)
    expect(state.draft?.source.features[0]).toEqual(original)
    expect(state.draft?.source.features[1]?.automatic).toBe(false)
    expect(screen.getByRole('textbox', { name: 'Input glyphs' })).toBeEnabled()
  })

  it('reloads source changes from document undo into the form', async () => {
    setup()
    await openVisual()
    state.active_source.features[0]!.code = 'sub B by B.alt;'
    state.generation += 1
    state.draft_revision += 1
    useDocumentStore.getState().noteGeneration(1, state.generation)
    await waitFor(() =>
      expect(screen.getByRole('textbox', { name: 'Input glyphs' })).toHaveValue(
        'B',
      ),
    )
  })
})
