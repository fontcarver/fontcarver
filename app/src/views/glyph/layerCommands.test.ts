import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { setTransport, type ApplyParams } from '../../api'
import { selectionOf } from '../../editor/selection'
import {
  isCommandEnabled,
  resetCommands,
  runCommand,
  setCommandErrorReporter,
} from '../../shell/commands'
import { commandForEvent } from '../../shell/shortcuts'
import { initialEditorState, useEditorStore } from '../../stores/editorStore'
import { initialUiState, useUiStore } from '../../stores/uiStore'
import {
  makeCell,
  makeContour,
  makeGlyph,
  makeGlyphDetail,
  makeLayer,
  makeLayerOutline,
  makeMaster,
  makePoint,
} from '../../test/fontFixtures'
import { createMockTransport, noChange } from '../../test/mockTransport'
import { buildScene } from './glyphScene'
import { registerLayerCommands } from './layerCommands'

function publish(
  background = false,
  editingBackground = false,
  locked = false,
) {
  const foreground = makeLayer({ contours: [makeContour([makePoint(0, 0)])] })
  const back = makeLayer({ id: 'bg', kind: { Background: 'm1' } })
  const glyph = makeGlyph({
    layers: background ? [foreground, back] : [foreground],
    locked,
  })
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
      outline: makeLayerOutline(editingBackground ? back : foreground),
      master: 'm1',
      masterGuides: [],
      reload: () => undefined,
    },
  })
}
const settle = () => new Promise((resolve) => setTimeout(resolve, 0))
beforeEach(() => {
  useUiStore.setState(initialUiState())
  useEditorStore.setState(initialEditorState())
  resetCommands()
  registerLayerCommands()
})
afterEach(() => {
  setTransport(undefined)
  setCommandErrorReporter(() => undefined)
})

describe('background editing commands', () => {
  it('binds Mod+B and switches between the current master and its background', async () => {
    publish(true)
    expect(
      commandForEvent(
        new KeyboardEvent('keydown', { key: 'b', ctrlKey: true }),
        false,
      ),
    ).toBe('layer.editBackground')
    runCommand('layer.editBackground')
    await settle()
    expect(useEditorStore.getState().editedLayer).toBe('bg')
    publish(true, true)
    runCommand('layer.editBackground')
    await settle()
    expect(useEditorStore.getState().editedLayer).toBe('layer-1')
  })
  it('creates an undoable background before selecting it', async () => {
    publish()
    const mock = createMockTransport({ 'doc.apply': () => noChange([42]) })
    setTransport(mock)
    runCommand('layer.editBackground')
    await settle()
    const command = (mock.calls[0]?.params as ApplyParams).command
    expect(command).toMatchObject({
      AddLayer: { glyph: 'a', layer: { kind: { Background: 'm1' } } },
    })
    if (!('AddLayer' in command)) throw new Error('expected layer creation')
    expect(useEditorStore.getState().editedLayer).toBe(
      command.AddLayer.layer.id,
    )
  })
  it('keeps the current layer when creating the background fails', async () => {
    publish()
    setTransport(
      createMockTransport({
        'doc.apply': () => {
          throw new Error('Refused')
        },
      }),
    )
    setCommandErrorReporter(() => undefined)
    runCommand('layer.editBackground')
    await settle()
    expect(useEditorStore.getState().editedLayer).toBeNull()
  })
  it('allows inspecting a locked glyph background but never creates one', () => {
    publish(false, false, true)
    expect(isCommandEnabled('layer.editBackground')).toBe(false)
    publish(true, false, true)
    expect(isCommandEnabled('layer.editBackground')).toBe(true)
  })
  it('enables selection copying only for foreground drawing objects', async () => {
    publish()
    const mock = createMockTransport({ 'doc.apply': () => noChange([42]) })
    setTransport(mock)
    expect(isCommandEnabled('layer.selectionToBackground')).toBe(false)
    useEditorStore
      .getState()
      .setSelection(selectionOf([{ kind: 'node', contour: 0, point: 0 }]))
    expect(isCommandEnabled('layer.selectionToBackground')).toBe(true)
    runCommand('layer.selectionToBackground')
    await settle()
    expect((mock.calls[0]?.params as ApplyParams).command).toMatchObject({
      CopySelectionToBackground: {
        from: 'layer-1',
        contours: [0],
        components: [],
      },
    })
    publish(true, true)
    useEditorStore
      .getState()
      .setSelection(selectionOf([{ kind: 'node', contour: 0, point: 0 }]))
    expect(isCommandEnabled('layer.selectionToBackground')).toBe(false)
  })
})
