import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { setTransport, type ApplyParams } from '../../../api'
import { getCommand, runCommand } from '../../../shell/commands'
import { initialEditorState, useEditorStore } from '../../../stores/editorStore'
import { initialUiState, useUiStore } from '../../../stores/uiStore'
import {
  makeCell,
  makeGlyph,
  makeLayer,
  makeLayerOutline,
  makeMaster,
} from '../../../test/fontFixtures'
import { createMockTransport, noChange } from '../../../test/mockTransport'
import { buildScene } from '../glyphScene'
import { makeGlyphDetail } from '../../../test/fontFixtures'
import { registerMetricsCommands } from './metricsCommands'

function transport() {
  const mock = createMockTransport({ 'doc.apply': () => noChange([42]) })
  setTransport(mock)
  return mock
}

function applied(mock: ReturnType<typeof transport>): ApplyParams[] {
  return mock.calls
    .filter((entry) => entry.method === 'doc.apply')
    .map((entry) => entry.params as ApplyParams)
}

/** Puts one glyph in the editor, with or without a resolved master. */
function editGlyph(master: string | null) {
  const layer = makeLayer()
  const glyph = makeGlyph({ layers: [layer] })
  useUiStore.getState().openTab({ kind: 'glyph', doc: 1, gid: 42 })
  useEditorStore.setState({
    activeGlyph: {
      doc: 1,
      gid: 42,
      glyph,
      cell: makeCell(),
      outline: makeLayerOutline(layer),
      master,
      masterGuides: [],
      reload: () => undefined,
      scene: buildScene({
        detail: makeGlyphDetail(glyph),
        cell: makeCell(),
        master: makeMaster(),
        masterId: 'm1',
        unitsPerEm: 1000,
      }),
    },
  })
}

beforeEach(() => {
  useEditorStore.setState(initialEditorState())
  useUiStore.setState(initialUiState())
  registerMetricsCommands()
})
afterEach(() => setTransport(undefined))

describe('metrics synchronisation commands', () => {
  it('updates the glyph in the selected master', async () => {
    const mock = transport()
    editGlyph('m1')
    await runCommand('glyph.updateMetrics')
    await vi.waitFor(() =>
      expect(applied(mock).map((entry) => entry.command)).toEqual([
        { UpdateMetrics: { glyph: 'a', master: 'm1' } },
      ]),
    )
  })

  it('never sends a null master from the current-master command', async () => {
    const mock = transport()
    // A document whose master has not resolved yet: `null` would mean *every*
    // master layer, so the command must refuse rather than resynchronise all.
    editGlyph(null)
    expect(getCommand('glyph.updateMetrics')?.when?.()).toBe(false)
    await runCommand('glyph.updateMetrics')
    await new Promise((resolve) => setTimeout(resolve, 10))
    expect(applied(mock)).toEqual([])
  })

  it('still updates every master when that is what was asked', async () => {
    const mock = transport()
    editGlyph(null)
    await runCommand('glyph.updateMetricsAll')
    await vi.waitFor(() =>
      expect(applied(mock).map((entry) => entry.command)).toEqual([
        { UpdateMetrics: { glyph: 'a', master: null } },
      ]),
    )
  })

  it('refuses both commands while the active text tab locks spacing', async () => {
    const mock = transport()
    editGlyph('m1')
    const tab = useUiStore.getState().tabs.at(-1)
    useUiStore.getState().updateTab(tab!.id, {
      textState: {
        ...(tab!.textState ?? {
          text: 'AV',
          activeIndex: null,
          mode: 'text' as const,
          master: 'm1',
          size: 128,
          features: [],
          direction: 'ltr' as const,
          script: null,
          language: null,
          previewHeight: 160,
          previewTheme: 'light' as const,
          previewCollapsed: false,
        }),
        kerningMode: 'locked',
      },
    })
    for (const id of ['glyph.updateMetrics', 'glyph.updateMetricsAll'])
      await runCommand(id)
    await new Promise((resolve) => setTimeout(resolve, 10))
    expect(applied(mock)).toEqual([])
  })
})
