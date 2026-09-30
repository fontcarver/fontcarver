import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import { setTransport, type ExportOptions, type ExportProfile } from '../api'
import { createMockTransport, type MockTransport } from '../test/mockTransport'
import { useExportProfileStore } from '../stores/exportProfileStore'
import { ExportProfiles } from './ExportProfiles'

function options(): ExportOptions {
  return {
    format: 'Woff2',
    variable: false,
    master: 'm1',
    remove_overlap: true,
    autohint: false,
    keep_tt_instructions: false,
    preserve_untouched_tables: true,
    production_names: false,
    table_subset: ['DSIG'],
    add_extrema: false,
    round_coordinates: false,
    decompose_components: false,
    subroutinize: false,
  }
}

/** A kernel double that keeps one collection per document. */
function projectTransport(
  rejectWrites = false,
): MockTransport & { stored: () => ExportProfile[] } {
  let profiles: ExportProfile[] = []
  const transport = createMockTransport({
    'export.profiles_get': () => ({
      profiles: structuredClone(profiles),
      variable_configs: [],
    }),
    'export.profiles_set': (params) => {
      if (rejectWrites) throw new Error('the project rejected the write')
      profiles = structuredClone(params.profiles)
      return { profiles: structuredClone(profiles), variable_configs: [] }
    },
  })
  return Object.assign(transport, { stored: () => profiles })
}

beforeEach(() => {
  vi.restoreAllMocks()
  useExportProfileStore.setState({
    docs: {},
    loading: {},
    error: null,
    detail: null,
  })
})

describe('ExportProfiles', () => {
  it('contains Enter within the profile controls and preserves keyboard activation', async () => {
    setTransport(projectTransport())
    const user = userEvent.setup()
    const confirm = vi.fn()
    render(
      <div
        onKeyDown={(event) => {
          if (event.key === 'Enter') confirm()
        }}
      >
        <ExportProfiles
          doc={1}
          options={options()}
          busy={false}
          onApply={vi.fn()}
        />
      </div>,
    )
    await user.type(screen.getByLabelText('Profile name'), 'Keyboard')
    await user.keyboard('{Enter}')
    expect(confirm).not.toHaveBeenCalled()
    screen.getByRole('button', { name: 'Save profile' }).focus()
    await user.keyboard('{Enter}')
    await waitFor(() =>
      expect(
        screen.getByRole('option', { name: 'Keyboard' }),
      ).toBeInTheDocument(),
    )
    expect(confirm).not.toHaveBeenCalled()
  })

  it('saves the current settings into the project and applies independent snapshots', async () => {
    const transport = projectTransport()
    setTransport(transport)
    const user = userEvent.setup()
    const source = options()
    const apply = vi.fn()
    render(
      <ExportProfiles doc={7} options={source} busy={false} onApply={apply} />,
    )
    await user.type(screen.getByLabelText('Profile name'), 'Web')
    await user.type(screen.getByLabelText('Output subfolder'), 'web')
    await user.click(screen.getByRole('button', { name: 'Save profile' }))
    await waitFor(() => expect(transport.stored()).toHaveLength(1))
    expect(transport.stored()[0]).toMatchObject({
      name: 'Web',
      subfolder: 'web',
      options: { format: 'Woff2' },
    })
    // The profile went to the document, not to browser storage.
    expect(localStorage.getItem('fontcarver.exportProfiles')).toBeNull()

    await user.selectOptions(
      screen.getByLabelText('Saved profile'),
      screen.getByRole('option', { name: 'Web' }),
    )
    await user.click(screen.getByRole('button', { name: 'Apply profile' }))
    expect(apply).toHaveBeenCalledOnce()
    const applied = apply.mock.calls[0]?.[0] as ExportProfile
    applied.options.table_subset?.push('TEST')
    await user.click(screen.getByRole('button', { name: 'Apply profile' }))
    expect(
      (apply.mock.calls[1]?.[0] as ExportProfile).options.table_subset,
    ).toEqual(['DSIG'])

    await user.click(screen.getByRole('button', { name: 'Delete profile' }))
    await waitFor(() => expect(transport.stored()).toEqual([]))
  })

  it('reports a duplicate name and an unsafe subfolder without writing', async () => {
    const transport = projectTransport()
    setTransport(transport)
    const user = userEvent.setup()
    render(
      <ExportProfiles
        doc={3}
        options={options()}
        busy={false}
        onApply={vi.fn()}
      />,
    )
    await user.type(screen.getByLabelText('Profile name'), 'Web')
    await user.click(screen.getByRole('button', { name: 'Save profile' }))
    await waitFor(() => expect(transport.stored()).toHaveLength(1))
    await user.type(screen.getByLabelText('Profile name'), 'web')
    await user.click(screen.getByRole('button', { name: 'Save profile' }))
    await waitFor(() =>
      expect(screen.getByRole('alert')).toHaveTextContent(
        'A profile with this name already exists.',
      ),
    )
    await user.clear(screen.getByLabelText('Profile name'))
    await user.type(screen.getByLabelText('Profile name'), 'Escape')
    await user.type(screen.getByLabelText('Output subfolder'), '../up')
    await user.click(screen.getByRole('button', { name: 'Save profile' }))
    await waitFor(() =>
      expect(screen.getByRole('alert')).toHaveTextContent(
        'Use a relative subfolder without',
      ),
    )
    expect(transport.stored()).toHaveLength(1)
  })

  it('shows a rejected write and keeps the selection usable', async () => {
    setTransport(projectTransport(true))
    const user = userEvent.setup()
    render(
      <ExportProfiles
        doc={4}
        options={options()}
        busy={false}
        onApply={vi.fn()}
      />,
    )
    await user.type(screen.getByLabelText('Profile name'), 'Web')
    await user.click(screen.getByRole('button', { name: 'Save profile' }))
    await waitFor(() =>
      expect(screen.getByRole('alert')).toHaveTextContent(
        'The project rejected these export settings.',
      ),
    )
    expect(
      screen.queryByRole('option', { name: 'Web' }),
    ).not.toBeInTheDocument()
  })

  it('discloses an unreadable stored collection and disables controls while busy', async () => {
    setTransport(
      createMockTransport({
        'export.profiles_get': () => {
          throw new Error('invalid stored export profiles: bad shape')
        },
      }),
    )
    render(
      <ExportProfiles doc={5} options={options()} busy onApply={vi.fn()} />,
    )
    await waitFor(() =>
      expect(screen.getByRole('alert')).toHaveTextContent('bad shape'),
    )
    expect(screen.getByLabelText('Profile name')).toBeDisabled()
    expect(screen.getByLabelText('Saved profile')).toBeDisabled()
    for (const button of screen.getAllByRole('button'))
      expect(button).toBeDisabled()
  })
})
