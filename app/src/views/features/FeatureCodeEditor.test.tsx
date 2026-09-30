import { redoDepth, undoDepth } from '@codemirror/commands'
import { diagnosticCount, forEachDiagnostic } from '@codemirror/lint'
import { EditorState } from '@codemirror/state'
import { EditorView } from '@codemirror/view'
import { act, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import {
  FeatureCodeEditor,
  type FeatureCodeEditorProps,
} from './FeatureCodeEditor'

function props(
  overrides: Partial<FeatureCodeEditorProps> = {},
): FeatureCodeEditorProps {
  return {
    value: 'feature kern { pos A V -80; } kern;',
    readOnly: false,
    label: 'Feature source',
    glyphNames: ['A', 'V'],
    classNames: [],
    diagnostics: [],
    focusRange: null,
    onChange: vi.fn(),
    onCompositionChange: vi.fn(),
    onHistory: vi.fn(),
    onSave: vi.fn(),
    ...overrides,
  }
}

function editor(): EditorView {
  const view = EditorView.findFromDOM(
    screen.getByRole('textbox', { name: 'Feature source' }),
  )
  if (!view) throw new Error('CodeMirror view was not mounted')
  return view
}

afterEach(() => vi.useRealTimers())

describe('FeatureCodeEditor', () => {
  it('previews a class with Alt-click or Alt+Enter without editing source', () => {
    const callbacks = props({
      value: 'sub @Upper by A;',
      onClassPreview: vi.fn(),
    })
    render(<FeatureCodeEditor {...callbacks} />)
    const target = screen.getByRole('textbox', { name: 'Feature source' })
    const coordinates = vi.spyOn(editor(), 'posAtCoords').mockReturnValue(6)
    fireEvent.mouseDown(target, {
      button: 0,
      altKey: true,
      clientX: 20,
      clientY: 20,
    })
    expect(callbacks.onClassPreview).toHaveBeenCalledExactlyOnceWith('Upper')
    coordinates.mockRestore()
    act(() => editor().dispatch({ selection: { anchor: 10 } }))
    fireEvent.keyDown(target, { key: 'Enter', altKey: true })
    expect(callbacks.onClassPreview).toHaveBeenCalledTimes(2)
    expect(callbacks.onClassPreview).toHaveBeenLastCalledWith('Upper')
    expect(callbacks.onChange).not.toHaveBeenCalled()
  })

  it('does not preview references inside comments and strings or during composition', () => {
    const callbacks = props({
      value: '# @Upper\nnameid 1 "@Upper";\nsub @Upper by A;',
      onClassPreview: vi.fn(),
    })
    render(<FeatureCodeEditor {...callbacks} />)
    const target = screen.getByRole('textbox', { name: 'Feature source' })
    for (const anchor of [5, 21]) {
      act(() => editor().dispatch({ selection: { anchor } }))
      fireEvent.keyDown(target, { key: 'Enter', altKey: true })
    }
    act(() => editor().dispatch({ selection: { anchor: 34 } }))
    fireEvent.keyDown(target, { key: 'Enter', altKey: true, isComposing: true })
    expect(callbacks.onClassPreview).not.toHaveBeenCalled()
  })

  it('hydrates and refreshes source without producing a document edit', () => {
    const callbacks = props()
    const { rerender } = render(<FeatureCodeEditor {...callbacks} />)
    expect(editor().state.doc.toString()).toBe(callbacks.value)
    expect(callbacks.onChange).not.toHaveBeenCalled()
    rerender(
      <FeatureCodeEditor
        {...callbacks}
        value={'# 中文😀\r\nfeature liga {} liga;'}
      />,
    )
    expect(editor().state.doc.toString()).toBe(
      '# 中文😀\nfeature liga {} liga;',
    )
    expect(callbacks.onChange).not.toHaveBeenCalled()
  })

  it('forwards edits once and retains no independent undo history', () => {
    const callbacks = props()
    render(<FeatureCodeEditor {...callbacks} />)
    act(() => editor().dispatch({ changes: { from: 0, insert: '# 字😀\n' } }))
    expect(callbacks.onChange).toHaveBeenCalledExactlyOnceWith(
      `# 字😀\n${callbacks.value}`,
    )
    expect(undoDepth(editor().state)).toBe(0)
    expect(redoDepth(editor().state)).toBe(0)
  })

  it('routes save and history to the document owner exactly once', () => {
    const callbacks = props()
    render(<FeatureCodeEditor {...callbacks} />)
    const target = screen.getByRole('textbox', { name: 'Feature source' })
    fireEvent.keyDown(target, { key: 'z', ctrlKey: true })
    fireEvent.keyDown(target, { key: 'z', ctrlKey: true, shiftKey: true })
    fireEvent.keyDown(target, { key: 'y', ctrlKey: true })
    fireEvent.keyDown(target, { key: 's', metaKey: true })
    expect(callbacks.onHistory).toHaveBeenCalledTimes(3)
    expect(callbacks.onHistory).toHaveBeenNthCalledWith(1, 'undo')
    expect(callbacks.onHistory).toHaveBeenNthCalledWith(2, 'redo')
    expect(callbacks.onHistory).toHaveBeenNthCalledWith(3, 'redo')
    expect(callbacks.onSave).toHaveBeenCalledOnce()
    expect(callbacks.onChange).not.toHaveBeenCalled()
  })

  it('keeps composition pending until its final text can settle', () => {
    vi.useFakeTimers()
    const callbacks = props()
    render(<FeatureCodeEditor {...callbacks} />)
    const target = screen.getByRole('textbox', { name: 'Feature source' })
    fireEvent.compositionStart(target)
    expect(callbacks.onCompositionChange).toHaveBeenLastCalledWith(true)
    fireEvent.keyDown(target, { key: 'z', ctrlKey: true, isComposing: true })
    expect(callbacks.onHistory).not.toHaveBeenCalled()
    fireEvent.keyDown(target, { key: 's', ctrlKey: true, isComposing: true })
    expect(callbacks.onSave).toHaveBeenCalledOnce()
    fireEvent.compositionEnd(target)
    expect(callbacks.onCompositionChange).toHaveBeenLastCalledWith(true)
    act(() => vi.advanceTimersByTime(60))
    expect(callbacks.onCompositionChange).toHaveBeenLastCalledWith(false)
  })

  it('routes native menu history input to the same document owner', () => {
    const callbacks = props()
    render(<FeatureCodeEditor {...callbacks} />)
    const target = screen.getByRole('textbox', { name: 'Feature source' })
    const event = new InputEvent('beforeinput', {
      bubbles: true,
      cancelable: true,
      inputType: 'historyUndo',
    })
    fireEvent(target, event)
    expect(event.defaultPrevented).toBe(true)
    expect(callbacks.onHistory).toHaveBeenCalledExactlyOnceWith('undo')
    expect(callbacks.onChange).not.toHaveBeenCalled()
  })

  it('honors read-only changes without destroying the view', () => {
    const callbacks = props({ readOnly: true })
    const { rerender } = render(<FeatureCodeEditor {...callbacks} />)
    const initialView = editor()
    expect(initialView.state.facet(EditorState.readOnly)).toBe(true)
    expect(initialView.contentDOM).toHaveAttribute('aria-readonly', 'true')
    rerender(<FeatureCodeEditor {...callbacks} readOnly={false} />)
    expect(editor()).toBe(initialView)
    expect(editor().state.facet(EditorState.readOnly)).toBe(false)
    expect(editor().contentDOM).toHaveAttribute('aria-readonly', 'false')
  })

  it('maps diagnostics and focus ranges using raw-source UTF-16 offsets', () => {
    const value = '# 字😀\r\npos A V -80;'
    const from = value.indexOf('A')
    const callbacks = props({
      value,
      diagnostics: [
        { from, to: from + 1, severity: 'error', message: 'Missing glyph' },
      ],
      focusRange: { from, to: from + 1, revision: 1 },
    })
    render(<FeatureCodeEditor {...callbacks} />)
    expect(diagnosticCount(editor().state)).toBe(1)
    forEachDiagnostic(editor().state, (_diagnostic, start, end) => {
      expect(editor().state.sliceDoc(start, end)).toBe('A')
    })
    expect(
      editor().state.sliceDoc(
        editor().state.selection.main.from,
        editor().state.selection.main.to,
      ),
    ).toBe('A')
    expect(callbacks.onChange).not.toHaveBeenCalled()
  })

  it('does not steal focus when an unchanged diagnostic range is recreated', () => {
    const callbacks = props({ focusRange: { from: 0, to: 7, revision: 1 } })
    const { rerender } = render(
      <>
        <FeatureCodeEditor {...callbacks} />
        <button>Elsewhere</button>
      </>,
    )
    act(() => screen.getByRole('button', { name: 'Elsewhere' }).focus())
    rerender(
      <>
        <FeatureCodeEditor
          {...callbacks}
          focusRange={{ from: 0, to: 7, revision: 1 }}
        />
        <button>Elsewhere</button>
      </>,
    )
    expect(screen.getByRole('button', { name: 'Elsewhere' })).toHaveFocus()
  })
})
