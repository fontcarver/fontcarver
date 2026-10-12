import { beforeEach, describe, expect, it } from 'vitest'

import { TOOLS } from '../editor/tools/registry'
import { useKeymapStore } from '../stores/keymapStore'

import { toolForShortcut, toolShortcut } from './toolShortcuts'

const key = (value: string, init: KeyboardEventInit = {}) =>
  new KeyboardEvent('keydown', { key: value, ...init })

beforeEach(() => useKeymapStore.setState({ overrides: {} }))

describe('tool shortcuts', () => {
  it('preserves every default tool key and default Shift fallback', () => {
    for (const tool of TOOLS) {
      expect(
        toolForShortcut(key(tool.key, { shiftKey: tool.shift }), false),
      ).toBe(tool.id)
    }
    expect(toolForShortcut(key('P', { shiftKey: true }), false)).toBe('pen')
    expect(toolForShortcut(key('F', { shiftKey: true }), false)).toBe('ellipse')
  })

  it('uses live overrides and disables null bindings', () => {
    useKeymapStore.setState({
      overrides: { 'tool.pen': 'U', 'tool.knife': null },
    })
    expect(toolShortcut('pen')).toBe('U')
    expect(toolShortcut('knife')).toBeNull()
    expect(toolForShortcut(key('u'), false)).toBe('pen')
    expect(toolForShortcut(key('p'), false)).toBeNull()
    expect(toolForShortcut(key('e'), false)).toBeNull()
    useKeymapStore.setState({ overrides: { 'tool.pen': 'W' } })
    expect(toolForShortcut(key('w'), false)).toBe('pen')
    expect(toolForShortcut(key('u'), false)).toBeNull()
  })

  it('requires exact modifiers on custom bindings', () => {
    useKeymapStore.setState({ overrides: { 'tool.pen': 'Mod+Shift+U' } })
    expect(toolForShortcut(key('u'), false)).toBeNull()
    expect(
      toolForShortcut(key('u', { ctrlKey: true, shiftKey: true }), false),
    ).toBe('pen')
    expect(
      toolForShortcut(key('u', { metaKey: true, shiftKey: true }), true),
    ).toBe('pen')
    expect(
      toolForShortcut(key('u', { ctrlKey: true, shiftKey: true }), true),
    ).toBeNull()
    useKeymapStore.setState({ overrides: { 'tool.pen': 'U' } })
    expect(toolForShortcut(key('U', { shiftKey: true }), false)).toBeNull()
  })

  it('gives explicit Shift bindings priority over default fallback', () => {
    useKeymapStore.setState({ overrides: { 'tool.pen': 'Shift+V' } })
    expect(toolForShortcut(key('V', { shiftKey: true }), false)).toBe('pen')
    expect(toolForShortcut(key('v'), false)).toBe('select')
  })

  it('accepts a supplied override map for reactive labels', () => {
    expect(toolShortcut('pen', { 'tool.pen': 'Alt+Q' })).toBe('Alt+Q')
    expect(toolShortcut('ellipse', {})).toBe('Shift+F')
  })
})
