import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import type { ExportOptions } from '../api'
import { useExportProfileStore } from '../stores/exportProfileStore'
import { ExportProfiles } from './ExportProfiles'

function options(): ExportOptions {
  const value = {
    format: 'Woff2' as const,
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
  return value
}

beforeEach(() => {
  vi.restoreAllMocks()
  localStorage.clear()
  useExportProfileStore.getState().reload()
})

describe('ExportProfiles', () => {
  it('contains Enter within the profile controls and preserves keyboard button activation', async () => {
    const user = userEvent.setup()
    const confirm = vi.fn()
    render(
      <div
        onKeyDown={(event) => {
          if (event.key === 'Enter') confirm()
        }}
      >
        <ExportProfiles options={options()} busy={false} onApply={vi.fn()} />
      </div>,
    )
    await user.type(screen.getByLabelText('Profile name'), 'Keyboard')
    await user.keyboard('{Enter}')
    expect(confirm).not.toHaveBeenCalled()
    screen.getByRole('button', { name: 'Save profile' }).focus()
    await user.keyboard('{Enter}')
    expect(screen.getByRole('option', { name: 'Keyboard' })).toBeInTheDocument()
    expect(confirm).not.toHaveBeenCalled()
  })

  it('saves current settings, restores after remount, and applies independent snapshots', async () => {
    const user = userEvent.setup()
    const source = options()
    const first = render(
      <ExportProfiles options={source} busy={false} onApply={vi.fn()} />,
    )
    await user.type(screen.getByLabelText('Profile name'), 'Web')
    await user.click(screen.getByRole('button', { name: 'Save profile' }))
    source.table_subset?.push('TEST')
    first.unmount()
    useExportProfileStore.setState({ profiles: [] })
    const apply = vi.fn()
    render(<ExportProfiles options={source} busy={false} onApply={apply} />)
    await user.selectOptions(
      screen.getByLabelText('Saved profile'),
      screen.getByRole('option', { name: 'Web' }),
    )
    await user.click(screen.getByRole('button', { name: 'Apply profile' }))
    expect(apply).toHaveBeenCalledWith(options())
    apply.mock.calls[0]?.[0].table_subset.push('TEST')
    await user.click(screen.getByRole('button', { name: 'Apply profile' }))
    expect(apply.mock.calls[1]?.[0]).toEqual(options())
    await user.click(screen.getByRole('button', { name: 'Delete profile' }))
    expect(
      screen.queryByRole('option', { name: 'Web' }),
    ).not.toBeInTheDocument()
    useExportProfileStore.getState().reload()
    expect(useExportProfileStore.getState().profiles).toEqual([])
  })

  it('shows failures explicitly and keeps a failed deletion selected', async () => {
    const user = userEvent.setup()
    useExportProfileStore.getState().save('Web', options())
    render(
      <ExportProfiles options={options()} busy={false} onApply={vi.fn()} />,
    )
    await user.type(screen.getByLabelText('Profile name'), 'web')
    await user.click(screen.getByRole('button', { name: 'Save profile' }))
    expect(screen.getByRole('alert')).toHaveTextContent(
      'A profile with this name already exists.',
    )
    await user.selectOptions(
      screen.getByLabelText('Saved profile'),
      screen.getByRole('option', { name: 'Web' }),
    )
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('Full')
    })
    await user.click(screen.getByRole('button', { name: 'Delete profile' }))
    expect(screen.getByRole('alert')).toHaveTextContent(
      'Profile storage is unavailable. Your changes were not saved.',
    )
    expect(screen.getByRole('button', { name: 'Apply profile' })).toBeEnabled()
  })

  it('discloses corrupt stored data on mount and disables controls while busy', () => {
    localStorage.setItem('fontcarver.exportProfiles', 'corrupt')
    render(<ExportProfiles options={options()} busy onApply={vi.fn()} />)
    expect(screen.getByRole('alert')).toHaveTextContent(
      'Saved export profiles could not be read.',
    )
    expect(screen.getByLabelText('Profile name')).toBeDisabled()
    expect(screen.getByLabelText('Saved profile')).toBeDisabled()
    for (const button of screen.getAllByRole('button'))
      expect(button).toBeDisabled()
  })
})
