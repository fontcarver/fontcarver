import { act, render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { setTransport, type ExportParams, type ExportReport } from '../api'
import { i18n } from '../i18n'
import { initialDocumentState, useDocumentStore } from '../stores/documentStore'
import { initialProgressState, useProgressStore } from '../stores/progressStore'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { cellFor } from '../test/glyphFixtures'
import { createMockTransport, makeSummary } from '../test/mockTransport'
import { ApiCallError } from '../transport'

import { ExportDialog } from './ExportDialog'

function makeReport(overrides: Partial<ExportReport> = {}): ExportReport {
  return {
    path: '/tmp/DejaVuSans.ttf',
    format: 'Ttf',
    bytes_written: 4096,
    warnings: [],
    tables: [],
    touched_glyphs: 1,
    timing_ms: 12,
    ...overrides,
  }
}

function setup(
  summary = makeSummary(),
  report: ExportReport | (() => ExportReport) = makeReport(),
) {
  const transport = createMockTransport({
    'session.export': () => (typeof report === 'function' ? report() : report),
    'glyphs.search': ({ query }) => [cellFor(7, { name: query })],
  })
  setTransport(transport)
  useSessionStore.setState({ docs: [summary] })
  useUiStore.getState().openTab({ kind: 'font', doc: summary.doc })
  useUiStore.getState().setDialog({ kind: 'export', doc: summary.doc })
  render(<ExportDialog summary={summary} />)
  return transport
}

beforeEach(() => {
  localStorage.clear()
  useUiStore.setState(initialUiState())
  useSessionStore.setState(initialSessionState())
  useDocumentStore.setState(initialDocumentState())
  useProgressStore.setState(initialProgressState())
})

afterEach(() => {
  setTransport(undefined)
})

describe('ExportDialog', () => {
  it('maps the form onto session.export parameters', async () => {
    const user = userEvent.setup()
    const transport = setup()

    await user.click(screen.getByRole('checkbox', { name: 'Remove overlap' }))
    await user.click(
      screen.getByRole('checkbox', { name: 'Preserve untouched tables' }),
    )
    await user.click(screen.getByRole('button', { name: 'Export' }))

    await waitFor(() => expect(transport.calls).toHaveLength(1))
    expect(transport.calls[0]?.method).toBe('session.export')
    expect(transport.calls[0]?.params).toEqual({
      doc: 1,
      path: '/fonts/DejaVuSans.ttf',
      options: {
        format: 'Ttf',
        master: 'm1',
        remove_overlap: true,
        keep_tt_instructions: true,
        preserve_untouched_tables: false,
        production_names: false,
        autohint: false,
        table_subset: null,
      },
    })
  })

  it('follows the format tab into the file extension and the options', async () => {
    const user = userEvent.setup()
    const transport = setup()

    await user.click(screen.getByRole('tab', { name: 'WOFF2' }))
    expect(screen.getByLabelText('Destination path')).toHaveValue(
      '/fonts/DejaVuSans.woff2',
    )

    await user.click(screen.getByRole('tab', { name: 'UFO' }))
    expect(screen.getByLabelText('Destination path')).toHaveValue(
      '/fonts/DejaVuSans.ufo',
    )
    // UFO is written by the source writer, so the sfnt options do not apply.
    expect(
      screen.queryByRole('checkbox', { name: 'Remove overlap' }),
    ).not.toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'Export' }))
    await waitFor(() => expect(transport.calls).toHaveLength(1))
    expect((transport.calls[0]?.params as ExportParams).options.format).toBe(
      'Ufo',
    )
  })

  it('offers the Designspace tab only for a multiple-master document', () => {
    setup()
    expect(
      screen.queryByRole('tab', { name: 'Designspace' }),
    ).not.toBeInTheDocument()

    useUiStore.setState(initialUiState())
    setup(
      makeSummary({
        masters: [
          { id: 'm1', name: 'Light' },
          { id: 'm2', name: 'Bold' },
        ],
      }),
    )
    expect(
      screen.getAllByRole('tab', { name: 'Designspace' }).length,
    ).toBeGreaterThan(0)
  })

  it('exports OTF without offering TrueType instruction retention', async () => {
    const user = userEvent.setup()
    const transport = setup()
    await user.click(screen.getByRole('tab', { name: 'OTF (CFF)' }))
    expect(screen.getByLabelText('Destination path')).toHaveValue(
      '/fonts/DejaVuSans.otf',
    )
    expect(
      screen.queryByRole('checkbox', { name: 'Keep TrueType instructions' }),
    ).not.toBeInTheDocument()
    expect(
      screen.getByText(/CFF outlines are exported without hinting/),
    ).toBeVisible()
    await user.click(screen.getByRole('button', { name: 'Export' }))
    await waitFor(() => expect(transport.calls).toHaveLength(1))
    expect((transport.calls[0]?.params as ExportParams).options).toMatchObject({
      format: 'Otf',
      master: 'm1',
      keep_tt_instructions: false,
    })
  })

  it('selects the new formats from the keyboard', async () => {
    const user = userEvent.setup()
    setup()
    screen.getByRole('tab', { name: 'TTF' }).focus()
    await user.keyboard('{ArrowRight}')
    expect(screen.getByRole('tab', { name: 'OTF (CFF)' })).toHaveFocus()
    expect(screen.getByLabelText('Destination path')).toHaveValue(
      '/fonts/DejaVuSans.otf',
    )
    await user.keyboard('{End}{ArrowLeft}')
    expect(screen.getByRole('tab', { name: 'PFB' })).toHaveFocus()
    expect(
      screen.getByText(/Type 1 cannot preserve OpenType layout/),
    ).toBeVisible()
  })

  it('exports all Glyphs masters but uses the selected master for SFD', async () => {
    const user = userEvent.setup()
    const transport = setup(
      makeSummary({
        masters: [
          { id: 'm1', name: 'Light' },
          { id: 'm2', name: 'Bold' },
        ],
      }),
    )
    await user.selectOptions(screen.getByLabelText('Master'), 'm2')
    await user.click(screen.getByRole('tab', { name: 'Glyphs' }))
    expect(screen.queryByLabelText('Master')).not.toBeInTheDocument()
    expect(screen.getByText('All masters are included.')).toBeVisible()
    await user.click(screen.getByRole('button', { name: 'Export' }))
    await waitFor(() => expect(transport.calls).toHaveLength(1))
    expect((transport.calls[0]?.params as ExportParams).options).toMatchObject({
      format: 'Glyphs',
      master: null,
    })
    await user.click(screen.getByRole('tab', { name: 'SFD' }))
    expect(screen.getByLabelText('Master')).toHaveValue('m2')
    await user.click(screen.getByRole('button', { name: 'Export' }))
    await waitFor(() => expect(transport.calls).toHaveLength(2))
    expect((transport.calls[1]?.params as ExportParams).options).toMatchObject({
      format: 'Sfd',
      master: 'm2',
    })
  })

  it.each(['PFA', 'PFB'])(
    'discloses Type 1 loss before exporting %s',
    async (label) => {
      const user = userEvent.setup()
      const transport = setup()
      await user.click(screen.getByRole('tab', { name: label }))
      expect(
        screen.getByText(/Type 1 cannot preserve OpenType layout/),
      ).toBeVisible()
      expect(screen.getByLabelText('Master')).toBeVisible()
      expect(
        screen.queryByRole('checkbox', { name: 'Preserve untouched tables' }),
      ).not.toBeInTheDocument()
      expect(transport.calls).toHaveLength(0)
    },
  )

  it('shows the busy line while the export runs and clears it afterwards', async () => {
    const user = userEvent.setup()
    let release = () => undefined as void
    const gate = new Promise<void>((resolve) => {
      release = () => resolve()
    })
    const transport = createMockTransport({
      'session.export': async () => {
        await gate
        return makeReport()
      },
    })
    setTransport(transport)
    const summary = makeSummary()
    useSessionStore.setState({ docs: [summary] })
    render(<ExportDialog summary={summary} />)

    await user.click(screen.getByRole('button', { name: 'Export' }))
    await waitFor(() =>
      expect(useProgressStore.getState().byDoc[1]).toBeDefined(),
    )

    release()
    await waitFor(() =>
      expect(useProgressStore.getState().byDoc[1]).toBeUndefined(),
    )
  })

  it('lists warnings with their affected glyphs as chips', async () => {
    const user = userEvent.setup()
    const transport = setup(
      makeSummary(),
      makeReport({
        warnings: [
          {
            code: 'hinting-dropped',
            message:
              'TrueType instructions were dropped because these glyphs changed: A, B',
          },
        ],
      }),
    )

    await user.click(screen.getByRole('button', { name: 'Export' }))

    expect(
      await screen.findByText(
        'All TrueType instructions were dropped from the exported font.',
      ),
    ).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'A' }))

    await waitFor(() =>
      expect([
        ...(useDocumentStore.getState().byDoc[1]?.selection ?? []),
      ]).toEqual([7]),
    )
    expect(transport.calls.at(-1)?.method).toBe('glyphs.search')
  })

  it('ignores a delayed warning lookup after another tab becomes active', async () => {
    const user = userEvent.setup()
    setup(
      makeSummary(),
      makeReport({
        warnings: [
          {
            code: 'hinting-dropped',
            message:
              'TrueType instructions were dropped because these glyphs changed: A',
          },
        ],
      }),
    )
    await user.click(screen.getByRole('button', { name: 'Export' }))
    await screen.findByRole('button', { name: 'A' })
    let resolve!: (cells: ReturnType<typeof cellFor>[]) => void
    const lookup = new Promise<ReturnType<typeof cellFor>[]>((yes) => {
      resolve = yes
    })
    setTransport(createMockTransport({ 'glyphs.search': () => lookup }))
    await user.click(screen.getByRole('button', { name: 'A' }))
    let next = ''
    await act(async () => {
      next = useUiStore.getState().openTab({ kind: 'font', doc: 2 })
      resolve([cellFor(7, { name: 'A' })])
      await lookup
    })
    expect(useUiStore.getState().activeTab).toBe(next)
    expect(useDocumentStore.getState().byDoc[1]?.focus ?? null).toBeNull()
  })

  it('caps the chips of a warning that names thousands of glyphs', async () => {
    const user = userEvent.setup()
    const names = Array.from({ length: 120 }, (_, index) => `g${index}`)
    setup(
      makeSummary(),
      makeReport({
        warnings: [
          {
            code: 'hinting-dropped',
            message: `TrueType instructions were dropped because these glyphs changed: ${names.join(', ')}`,
          },
        ],
      }),
    )

    await user.click(screen.getByRole('button', { name: 'Export' }))

    await screen.findByRole('button', { name: 'g0' })
    expect(screen.getAllByTitle(/^Select g\d+ in the Font view$/)).toHaveLength(
      50,
    )
    expect(screen.getByText('70 more')).toBeInTheDocument()
    expect(
      screen.queryByRole('button', { name: 'g119' }),
    ).not.toBeInTheDocument()
  })

  it('reports a failed export by its kernel error code', async () => {
    const user = userEvent.setup()
    const raw = 'failed to write `/fonts/DejaVuSans.ttf`'
    const transport = createMockTransport({
      'session.export': () => {
        throw new ApiCallError('export', raw)
      },
    })
    setTransport(transport)
    const summary = makeSummary()
    useSessionStore.setState({ docs: [summary] })
    render(<ExportDialog summary={summary} />)

    await user.click(screen.getByRole('button', { name: 'Export' }))

    // The summary the code maps to, with the kernel's own wording kept behind
    // the same "Details" disclosure the toasts use.
    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent(i18n.t('errors.export'))
    expect(within(alert).getByText(raw)).not.toBeVisible()
    await user.click(within(alert).getByText('Details'))
    expect(within(alert).getByText(raw)).toBeVisible()
  })

  it('keeps a failure the catalogue cannot translate readable', async () => {
    const user = userEvent.setup()
    const transport = createMockTransport({
      'session.export': () => {
        throw new Error('the kernel went away')
      },
    })
    setTransport(transport)
    const summary = makeSummary()
    useSessionStore.setState({ docs: [summary] })
    render(<ExportDialog summary={summary} />)

    await user.click(screen.getByRole('button', { name: 'Export' }))

    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent('the kernel went away')
    expect(within(alert).queryByText('Details')).not.toBeInTheDocument()
  })
})
