import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { TOOLS } from '../../editor/tools/registry'
import {
  getCommand,
  isCommandEnabled,
  listCommands,
  resetCommands,
  runCommand,
} from '../../shell/commands'
import { initialEditorState, useEditorStore } from '../../stores/editorStore'
import {
  makeCell,
  makeGlyph,
  makeGlyphDetail,
  makeLayer,
  makeLayerOutline,
  makeMaster,
} from '../../test/fontFixtures'

import { buildScene } from './glyphScene'
import { registerToolCommands } from './toolCommands'

function publish() {
  const layer = makeLayer()
  const glyph = makeGlyph({ layers: [layer] })
  useEditorStore.setState({
    ...initialEditorState(),
    activeGlyph: {
      doc: 1,
      gid: 42,
      glyph,
      cell: makeCell(),
      scene: buildScene({
        detail: makeGlyphDetail(glyph),
        cell: makeCell(),
        master: makeMaster(),
        masterId: 'm1',
        unitsPerEm: 1000,
      }),
      outline: makeLayerOutline(layer),
      master: 'm1',
      masterGuides: [],
      reload: () => undefined,
    },
  })
}

beforeEach(() => {
  localStorage.clear()
  resetCommands()
  useEditorStore.setState(initialEditorState())
  registerToolCommands()
})

afterEach(() => {
  resetCommands()
  useEditorStore.setState(initialEditorState())
})

describe('every tool is reachable from the command palette', () => {
  it('registers one command per tool of the strip', () => {
    const ids = listCommands().map((command) => command.id)
    for (const tool of TOOLS) expect(ids).toContain(`tool.${tool.id}`)
  })

  it('is disabled until an Edit tab is showing a glyph', () => {
    expect(isCommandEnabled('tool.pen')).toBe(false)
    publish()
    expect(isCommandEnabled('tool.pen')).toBe(true)
  })

  it('selects the tool and drops a borrowed one', () => {
    publish()
    useEditorStore.getState().borrowTool('hand')
    expect(runCommand('tool.knife')).toBe(true)
    expect(useEditorStore.getState().tool).toBe('knife')
    expect(useEditorStore.getState().borrowedTool).toBeNull()
  })

  it('shows the live binding and the gesture hint beside the name', () => {
    publish()
    expect(getCommand('tool.pen')?.detail?.()).toBe('P')
    const eraser = getCommand('tool.eraser')?.detail?.() ?? ''
    expect(eraser).toContain('Shift+E')
    expect(eraser).toContain('Shift leaves a gap')
  })

  it('claims no keyboard shortcut of its own, leaving tap and hold alone', () => {
    for (const command of listCommands())
      if (command.id.startsWith('tool.'))
        expect(command.shortcut).toBeUndefined()
  })
})
