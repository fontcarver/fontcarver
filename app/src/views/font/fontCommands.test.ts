import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { GlyphCell, GlyphListParams } from '../../api'
import { setTransport } from '../../api'
import {
  isCommandEnabled,
  resetCommands,
  runCommand,
} from '../../shell/commands'
import {
  initialDocumentState,
  useDocumentStore,
} from '../../stores/documentStore'
import { initialUiState, useUiStore } from '../../stores/uiStore'
import { createMockTransport, makeSummary } from '../../test/mockTransport'
import { initialSessionState, useSessionStore } from '../../stores/sessionStore'

import { registerFontViewCommands } from './fontCommands'
import { forgetGlyphNames, rememberGlyphNames } from './glyphNames'

function cellFor(gid: number): GlyphCell {
  return {
    gid,
    name: `g${gid}`,
    unicodes: [],
    width: 500,
    bbox: null,
    path: null,
    color_label: null,
    export: true,
    has_components: false,
    is_empty: false,
    generation: 0,
  }
}

/** Opens a Font tab for the mock document and returns its id. */
function openFontTab(): number {
  const summary = makeSummary()
  useSessionStore.setState({ ...initialSessionState(), docs: [summary] })
  useUiStore.getState().openTab({ kind: 'font', doc: summary.doc })
  useDocumentStore.getState().ensure(summary.doc)
  return summary.doc
}

beforeEach(() => {
  localStorage.clear()
  resetCommands()
  useUiStore.setState(initialUiState())
  useSessionStore.setState(initialSessionState())
  useDocumentStore.setState(initialDocumentState())
  registerFontViewCommands()
})

afterEach(() => {
  setTransport(undefined)
  resetCommands()
  vi.unstubAllGlobals()
})

describe('font view commands', () => {
  it('selects every glyph of the current filter, paging the kernel', async () => {
    const doc = openFontTab()
    useDocumentStore.getState().setMatchCount(doc, 2_500)
    const transport = createMockTransport({
      'glyphs.list': (params: GlyphListParams) => ({
        total: 2_500,
        items: Array.from(
          { length: Math.min(params.limit, 2_500 - params.offset) },
          (_, index) => cellFor(params.offset + index),
        ),
      }),
    })
    setTransport(transport)

    await runCommandAndSettle('edit.selectAll')

    expect(useDocumentStore.getState().byDoc[doc]?.selection.size).toBe(2_500)
    expect(
      transport.calls
        .filter((entry) => entry.method === 'glyphs.list')
        .map((entry) => (entry.params as GlyphListParams).offset),
    ).toEqual([0, 2_000])
  })

  it('copies the names of the selection to the clipboard', async () => {
    const doc = openFontTab()
    rememberGlyphNames(doc, [cellFor(1), cellFor(2), cellFor(3)])
    useDocumentStore.getState().setSelection(doc, [1, 3])
    const writeText = vi.fn(async () => undefined)
    vi.stubGlobal('navigator', { clipboard: { writeText } })

    expect(isCommandEnabled('edit.copyGlyphNames')).toBe(true)
    await runCommandAndSettle('edit.copyGlyphNames')

    expect(writeText).toHaveBeenCalledWith('g1 g3')
    expect(useUiStore.getState().toasts[0]?.message).toBe(
      'Copied 2 glyph names',
    )
    forgetGlyphNames(doc)
  })

  it('gates the selection commands on a Font tab with a selection', () => {
    expect(isCommandEnabled('edit.selectAll')).toBe(false)
    expect(isCommandEnabled('edit.copyGlyphNames')).toBe(false)

    const doc = openFontTab()
    expect(isCommandEnabled('edit.selectAll')).toBe(true)
    expect(isCommandEnabled('edit.copyGlyphNames')).toBe(false)

    useDocumentStore.getState().setSelection(doc, [7])
    expect(isCommandEnabled('edit.copyGlyphNames')).toBe(true)
    expect(isCommandEnabled('edit.selectNone')).toBe(true)

    runCommand('edit.selectNone')
    expect(useDocumentStore.getState().byDoc[doc]?.selection.size).toBe(0)
  })
})

/** Runs a command and lets its asynchronous body settle. */
async function runCommandAndSettle(id: string): Promise<void> {
  expect(runCommand(id)).toBe(true)
  for (let tick = 0; tick < 10; tick += 1) await Promise.resolve()
  await new Promise((resolve) => setTimeout(resolve, 0))
}
