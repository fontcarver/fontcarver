import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { setTransport, type FeatureState } from '../../api'
import { useDocumentStore } from '../../stores/documentStore'
import { useUiStore } from '../../stores/uiStore'
import {
  createMockTransport,
  makeSummary,
  type MockHandlers,
} from '../../test/mockTransport'

import { FeaturesView } from './FeaturesView'
import { featureState, featureValidation } from './featureTestFixtures'

/** A document whose `smcp` entry is generated, plus a manual `ss03` entry. */
function stateWithEntries(): FeatureState {
  const base = featureState()
  return {
    ...base,
    active_source: {
      ...base.active_source,
      features: [
        {
          id: 'fontcarver-generated-smcp',
          tag: 'smcp',
          code: 'sub \\a by \\a.sc;',
          automatic: true,
          disabled: false,
          notes: '',
        },
        {
          id: 'user-ss03',
          tag: 'ss03',
          code: 'sub \\a by \\a.ss03;',
          automatic: false,
          disabled: false,
          notes: '',
        },
      ],
    },
    active_settings: {
      ...base.active_settings,
      small_caps: true,
      generated: [
        {
          generator: 'SmallCaps',
          source_id: 'fontcarver-generated-smcp',
          glyphs: ['a', 'a.sc'],
        },
      ],
    },
    automatic_entries: [
      { name: 'smcp', generator: 'SmallCaps', kind: 'Feature' },
      { name: 'ss03', generator: 'StylisticSets', kind: 'Feature' },
      { name: 'locl', generator: 'Localized', kind: 'Feature' },
      { name: 'Uppercase', generator: 'Classes', kind: 'Class' },
    ],
  }
}

let state: FeatureState
beforeEach(() => {
  state = stateWithEntries()
  useDocumentStore.getState().ensure(1)
  useDocumentStore.getState().setMaster(1, 'm1')
  useUiStore.setState({
    tabs: [{ id: 'features:1', doc: 1, kind: 'features' }],
    activeTab: 'features:1',
  })
})
afterEach(() => setTransport(undefined))

function setup(handlers: MockHandlers = {}) {
  const transport = createMockTransport({
    'features.get': () => structuredClone(state),
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
    'features.freeze': () => structuredClone(state),
    'glyphs.list': () => ({ total: 2, items: [] }),
    ...handlers,
  })
  setTransport(transport)
  render(<FeaturesView summary={makeSummary()} />)
  return transport
}

async function selectEntry(label: string) {
  fireEvent.click(await screen.findByRole('button', { name: label }))
  await waitFor(() =>
    expect(screen.getByRole('checkbox', { name: 'Automatic' })).toBeTruthy(),
  )
}

describe('per-entry automatic switch', () => {
  it('shows a generated entry as automatic and freezes only that entry', async () => {
    const transport = setup()
    await selectEntry('smcp')
    const toggle = screen.getByRole('checkbox', { name: 'Automatic' })
    expect(toggle).toBeChecked()
    fireEvent.click(toggle)
    await waitFor(() =>
      expect(
        transport.calls.find(({ method }) => method === 'features.freeze'),
      ).toBeTruthy(),
    )
    const call = transport.calls.find(
      ({ method }) => method === 'features.freeze',
    )
    expect(call?.params).toMatchObject({
      generator: 'SmallCaps',
      entry: 'smcp',
    })
  })

  it('asks before replacing a manual entry with generated source', async () => {
    const transport = setup()
    await selectEntry('ss03')
    const toggle = screen.getByRole('checkbox', { name: 'Automatic' })
    expect(toggle).not.toBeChecked()
    fireEvent.click(toggle)
    const dialog = await screen.findByRole('dialog', {
      name: 'Generate this entry automatically?',
    })
    expect(dialog).toHaveTextContent('ss03')
    // Cancelling leaves the draft untouched.
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }))
    await waitFor(() =>
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument(),
    )
    expect(
      transport.calls.some(({ method }) => method === 'features.set_draft'),
    ).toBe(false)

    fireEvent.click(screen.getByRole('checkbox', { name: 'Automatic' }))
    fireEvent.click(
      await screen.findByRole('button', {
        name: 'Replace with generated source',
      }),
    )
    await waitFor(() =>
      expect(
        transport.calls.find(({ method }) => method === 'features.set_draft'),
      ).toBeTruthy(),
    )
    const draft = transport.calls.find(
      ({ method }) => method === 'features.set_draft',
    )?.params as {
      source: { features: { tag: string }[] }
      settings: { stylistic_sets: boolean }
    }
    expect(draft.settings.stylistic_sets).toBe(true)
    expect(draft.source.features.map((block) => block.tag)).toEqual(['smcp'])
  })

  it('disables the switch with a reason when no recipe owns the entry', async () => {
    setup()
    await selectEntry('Setup')
    const toggle = screen.getByRole('checkbox', { name: 'Automatic' })
    expect(toggle).toBeDisabled()
    const reason = 'No automatic recipe can generate an entry with this name.'
    expect(toggle.closest('label')).toHaveAttribute('title', reason)
    // The reason is readable without hover and announced with the control.
    expect(screen.getByText(reason)).toBeInTheDocument()
    expect(
      document.getElementById(toggle.getAttribute('aria-describedby') ?? ''),
    ).toHaveTextContent(reason)
  })
})
