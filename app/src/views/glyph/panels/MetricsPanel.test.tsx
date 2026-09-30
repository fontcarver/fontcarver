import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { setTransport, type ApplyParams, type MetricsKeys } from '../../../api'
import { i18n } from '../../../i18n'
import catalog from '../../../i18n/en.json'
import { setCommandErrorReporter } from '../../../shell/commands'
import { initialEditorState, useEditorStore } from '../../../stores/editorStore'
import { initialUiState, useUiStore } from '../../../stores/uiStore'
import {
  makeCell,
  makeGlyph,
  makeGlyphDetail,
  makeLayer,
  makeLayerOutline,
  makeMaster,
} from '../../../test/fontFixtures'
import {
  createMockTransport,
  noChange,
  type MockTransport,
} from '../../../test/mockTransport'
import { buildScene } from '../glyphScene'
import { initialTextState } from '../../text/textState'

import { MetricsPanel } from './MetricsPanel'

const keys: MetricsKeys = {
  left: '=H',
  right: null,
  width: null,
  vertical_origin: '=ideograph',
  vertical_width: null,
}

function show(local?: MetricsKeys) {
  const layer = { ...makeLayer(), metrics_keys: local }
  const glyph = makeGlyph({ layers: [layer], metrics_keys: keys })
  const outline = makeLayerOutline(layer, { lsb: 40, rsb: 40, width: 600 })
  const reload = vi.fn()
  render(
    <MetricsPanel
      target={{ doc: 1, glyph: glyph.name, layer: layer.id }}
      gid={42}
      glyph={glyph}
      outline={outline}
      master="m1"
      masterGuides={[]}
      cell={makeCell()}
      reload={reload}
      scene={buildScene({
        detail: makeGlyphDetail(glyph),
        cell: makeCell(),
        master: makeMaster(),
        masterId: 'm1',
        unitsPerEm: 1000,
      })}
    />,
  )
  return reload
}

function setup(): MockTransport {
  const transport = createMockTransport({
    'doc.metrics_resolve': () => ({
      lsb: 100,
      rsb: 40,
      width: 660,
      source: 'Key',
    }),
    'doc.apply': () => noChange([42]),
  })
  setTransport(transport)
  return transport
}

function applied(transport: MockTransport): ApplyParams[] {
  return transport.calls
    .filter((call) => call.method === 'doc.apply')
    .map((call) => call.params as ApplyParams)
}

beforeEach(() => {
  i18n.addResourceBundle(
    'en',
    'translation',
    { panel: { metrics: catalog.panel.metrics } },
    true,
    true,
  )
  useEditorStore.setState(initialEditorState())
  useUiStore.setState(initialUiState())
})

afterEach(() => {
  setTransport(undefined)
  setCommandErrorReporter(() => undefined)
})

describe('metrics synchronization', () => {
  it('prevents spacing changes when the active text tab locks metrics', async () => {
    const transport = setup()
    useUiStore.getState().openTab({
      kind: 'glyph',
      doc: 1,
      gid: 42,
      textState: { ...initialTextState('m1', 'AV'), kerningMode: 'locked' },
    })
    show()
    expect(await screen.findByText('out of sync')).toBeVisible()
    for (const label of [
      'advance',
      'LSB',
      'RSB',
      'LSB key',
      'RSB key',
      'advance key',
    ])
      expect(screen.getByLabelText(label)).toBeDisabled()
    for (const name of ['Update current master', 'Update all masters']) {
      const button = screen.getByRole('button', { name })
      expect(button).toBeDisabled()
      await userEvent.click(button)
    }
    expect(applied(transport)).toHaveLength(0)
  })

  it('updates current and all masters through undoable backend commands', async () => {
    const transport = setup()
    const reload = show()
    expect(await screen.findByText('out of sync')).toBeVisible()
    await userEvent.click(
      screen.getByRole('button', { name: 'Update current master' }),
    )
    await userEvent.click(
      screen.getByRole('button', { name: 'Update all masters' }),
    )
    await waitFor(() => expect(applied(transport)).toHaveLength(2))
    expect(applied(transport).map((entry) => entry.command)).toEqual([
      { UpdateMetrics: { glyph: 'a', master: 'm1' } },
      { UpdateMetrics: { glyph: 'a', master: null } },
    ])
    expect(reload).toHaveBeenCalledTimes(2)
  })

  it('shows synchronized status but keeps all-master synchronization available', async () => {
    setTransport(
      createMockTransport({
        'doc.metrics_resolve': () => ({
          lsb: 40,
          rsb: 40,
          width: 600,
          source: 'Key',
        }),
      }),
    )
    show()
    expect(await screen.findByText('In sync')).toBeVisible()
    expect(
      screen.getByRole('button', { name: 'Update current master' }),
    ).toBeDisabled()
    expect(
      screen.getByRole('button', { name: 'Update all masters' }),
    ).toBeEnabled()
  })

  it('shows resolution failures and blocks stale current-master updates', async () => {
    setTransport(
      createMockTransport({
        'doc.metrics_resolve': () => {
          throw new Error('metrics cycle: a → H → a')
        },
      }),
    )
    show()
    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent(
      'Could not resolve metrics: metrics cycle: a → H → a',
    )
    expect(
      screen.getByRole('button', { name: 'Update current master' }),
    ).toBeDisabled()
  })

  it('reports command refusal and refetches the model', async () => {
    const reporter = vi.fn()
    setCommandErrorReporter(reporter)
    setTransport(
      createMockTransport({
        'doc.metrics_resolve': () => ({
          lsb: 100,
          rsb: 40,
          width: 660,
          source: 'Key',
        }),
        'doc.apply': () => {
          throw new Error('conflicting metrics keys')
        },
      }),
    )
    const reload = show()
    await userEvent.click(
      await screen.findByRole('button', { name: 'Update all masters' }),
    )
    await waitFor(() => expect(reporter).toHaveBeenCalledOnce())
    expect(reload).toHaveBeenCalledOnce()
  })

  it('restores a rejected formula draft after reporting the failure', async () => {
    const reporter = vi.fn()
    setCommandErrorReporter(reporter)
    setTransport(
      createMockTransport({
        'doc.metrics_resolve': () => ({
          lsb: 40,
          rsb: 40,
          width: 600,
          source: 'Key',
        }),
        'doc.apply': () => {
          throw new Error('invalid formula')
        },
      }),
    )
    show()
    const field = screen.getByLabelText('LSB key')
    await userEvent.clear(field)
    await userEvent.type(field, '=H+{Enter}')
    await waitFor(() => expect(reporter).toHaveBeenCalledOnce())
    expect(field).toHaveValue('=H')
  })

  it('shows the effective local formula and changes its scope without dropping unrelated keys', async () => {
    const transport = setup()
    const local = {
      ...keys,
      left: '==O',
      vertical_origin: null,
      width: '==H+100',
    }
    show(local)
    const field = screen.getByLabelText('LSB key')
    expect(field).toHaveValue('==O')
    await userEvent.clear(field)
    await userEvent.type(field, '=n{Enter}')
    await waitFor(() => expect(applied(transport)).toHaveLength(1))
    expect(applied(transport)[0]?.command).toEqual({
      Batch: [
        { SetMetricsKeys: { glyph: 'a', keys: { ...keys, left: '=n' } } },
        {
          SetLayerMetricsKeys: {
            glyph: 'a',
            layer: 'layer-1',
            keys: { ...local, left: null },
          },
        },
      ],
    })
  })
})
