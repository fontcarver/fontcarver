import { EditorView } from '@codemirror/view'
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from '@testing-library/react'
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

let state: FeatureState
beforeEach(() => {
  state = featureState()
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
    'glyphs.list': () => ({ total: 2, items: [] }),
    ...handlers,
  })
  setTransport(transport)
  render(<FeaturesView summary={makeSummary()} />)
  return transport
}
function editor() {
  const view = EditorView.findFromDOM(
    screen.getByRole('textbox', { name: 'Feature source code' }),
  )
  if (!view) throw new Error('Missing editor')
  return view
}

describe('FeaturesView', () => {
  it('opens the source tree without materializing a draft or dirtying the font', async () => {
    const transport = setup()
    await screen.findByRole('textbox', { name: 'Feature source code' })
    expect(editor().state.doc.toString()).toBe('# setup')
    expect(
      transport.calls.some(({ method }) => method === 'features.set_draft'),
    ).toBe(false)
    expect(screen.getByRole('status')).toHaveTextContent('Applied source')
  })
  it('persists invalid Unicode source before checking while the active source stays unchanged', async () => {
    setup({
      'features.validate': () => ({
        ...featureValidation(state),
        valid: false,
        diagnostics: [
          {
            severity: 'Error',
            code: 'parse',
            message: 'Invalid Unicode rule',
            source_id: 'p1',
            span: { start: 0, end: 3 },
            generated: false,
            master: 'm1',
          },
        ],
      }),
    })
    await screen.findByRole('textbox', { name: 'Feature source code' })
    act(() =>
      editor().dispatch({
        changes: {
          from: 0,
          to: editor().state.doc.length,
          insert: '# 字😀 invalid {',
        },
      }),
    )
    fireEvent.click(screen.getByRole('button', { name: 'Check source' }))
    await screen.findByText('Invalid Unicode rule')
    expect(state.draft?.source.prefixes[0]?.code).toBe('# 字😀 invalid {')
    expect(state.active_source.prefixes[0]?.code).toBe('# setup')
    expect(screen.getByRole('status')).toHaveTextContent('Draft saved')
  })
  it('navigates a diagnostic to its stable source block and UTF-16 range', async () => {
    setup({
      'features.validate': () => ({
        ...featureValidation(state),
        valid: false,
        diagnostics: [
          {
            severity: 'Error',
            code: 'parse',
            message: 'Check this rule',
            source_id: 'p2',
            span: { start: 7, end: 11 },
            generated: false,
            master: 'm1',
          },
        ],
      }),
    })
    await screen.findByRole('textbox', { name: 'Feature source code' })
    fireEvent.click(screen.getByRole('button', { name: 'Check source' }))
    fireEvent.click(
      await screen.findByRole('button', { name: /Check this rule/ }),
    )
    await waitFor(() =>
      expect(editor().state.doc.toString()).toBe('# 中文😀\nrule'),
    )
    expect(editor().state.selection.main.from).toBe(7)
    expect(editor().state.selection.main.to).toBe(11)
  })
  it('shows generated validation source read-only before activation', async () => {
    setup({
      'features.validate': () => {
        const result = featureValidation(state)
        result.generated_source = {
          ...result.generated_source,
          features: [
            {
              id: 'g1',
              tag: 'smcp',
              code: 'sub A by A.sc;',
              automatic: true,
              disabled: false,
              notes: '',
            },
          ],
        }
        result.generated_settings = {
          ...result.generated_settings,
          small_caps: true,
          generated: [
            { generator: 'SmallCaps', source_id: 'g1', glyphs: ['A', 'A.sc'] },
          ],
        }
        return result
      },
    })
    await screen.findByRole('textbox', { name: 'Feature source code' })
    fireEvent.click(screen.getByRole('button', { name: 'Check source' }))
    const tree = screen.getByRole('navigation', { name: 'Feature sources' })
    fireEvent.click(await within(tree).findByRole('button', { name: 'smcp' }))
    await waitFor(() =>
      expect(editor().state.doc.toString()).toBe('sub A by A.sc;'),
    )
    expect(
      screen.getByRole('textbox', { name: 'Feature source code' }),
    ).toHaveAttribute('contenteditable', 'false')
    expect(
      screen.getByRole('button', { name: 'Freeze into editable source' }),
    ).toBeDisabled()
    expect(state.draft).toBeNull()
  })
  it('keeps imported sources read-only and requires the conversion workflow', async () => {
    state = {
      ...state,
      authority: 'ImportedTables',
      active_source: {
        ...state.active_source,
        source_of_truth: 'ImportedTables',
      },
    }
    setup()
    await screen.findByRole('textbox', { name: 'Feature source code' })
    expect(
      screen.getByRole('button', { name: 'Compile & preview' }),
    ).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Add prefix' })).toBeDisabled()
    expect(
      screen.getByRole('textbox', { name: 'Feature source code' }),
    ).toHaveAttribute('contenteditable', 'false')
    expect(
      screen.getByText(/Inspect and convert the complete layout/),
    ).toBeInTheDocument()
  })
  it('cannot freeze a generated preview that changed under a stable source ID', async () => {
    const generated = {
      id: 'g1',
      tag: 'smcp',
      code: 'sub A by A.sc;',
      automatic: true,
      disabled: false,
      notes: '',
    }
    state.active_source.features = [generated]
    state.active_settings = {
      ...state.active_settings,
      small_caps: true,
      generated: [
        { generator: 'SmallCaps', source_id: 'g1', glyphs: ['A', 'A.sc'] },
      ],
    }
    setup({
      'features.validate': () => {
        const result = featureValidation(state)
        return {
          ...result,
          generated_source: {
            ...result.generated_source,
            features: [{ ...generated, code: 'sub A by A.sc; sub B by B.sc;' }],
          },
          generated_settings: {
            ...result.generated_settings,
            generated: [
              {
                generator: 'SmallCaps',
                source_id: 'g1',
                glyphs: ['A', 'A.sc', 'B', 'B.sc'],
              },
            ],
          },
        }
      },
    })
    await screen.findByRole('textbox', { name: 'Feature source code' })
    fireEvent.click(screen.getByRole('button', { name: 'Check source' }))
    await screen.findByText('Source compiles in 1 master.')
    fireEvent.click(
      within(
        screen.getByRole('navigation', { name: 'Feature sources' }),
      ).getByRole('button', { name: 'smcp' }),
    )
    await waitFor(() => expect(editor().state.doc.toString()).toContain('B.sc'))
    expect(
      screen.getAllByRole('button', { name: 'Freeze into editable source' }),
    ).toHaveLength(2)
    for (const button of screen.getAllByRole('button', {
      name: 'Freeze into editable source',
    }))
      expect(button).toBeDisabled()
  })
})
