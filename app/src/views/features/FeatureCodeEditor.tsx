import {
  autocompletion,
  closeBrackets,
  closeBracketsKeymap,
  completionKeymap,
} from '@codemirror/autocomplete'
import { defaultKeymap } from '@codemirror/commands'
import { bracketMatching, syntaxHighlighting } from '@codemirror/language'
import { lintGutter, setDiagnostics, type Diagnostic } from '@codemirror/lint'
import { Annotation, Compartment, EditorState } from '@codemirror/state'
import {
  EditorView,
  highlightActiveLine,
  keymap,
  lineNumbers,
} from '@codemirror/view'
import { classHighlighter } from '@lezer/highlight'
import { useLayoutEffect, useRef } from 'react'

import {
  createFeatureCompletion,
  featureLanguage,
  sourceOffsetToEditorOffset,
} from './featureLanguage'
import styles from './FeatureCodeEditor.module.css'

/** Located diagnostics use UTF-16 offsets in the unnormalized source string. */
export interface FeatureEditorDiagnostic {
  from: number
  to: number
  severity: 'error' | 'warning' | 'info'
  message: string
}

/** Controlled FEA editor; the document owns persistence, saving and history. */
export interface FeatureCodeEditorProps {
  value: string
  readOnly: boolean
  label: string
  glyphNames: readonly string[]
  loadGlyphNames?: (query: string) => Promise<readonly string[]>
  classNames: readonly string[]
  diagnostics: readonly FeatureEditorDiagnostic[]
  focusRange: { from: number; to: number; revision: number } | null
  phrases?: Record<string, string>
  onChange: (value: string) => void
  onCompositionChange: (composing: boolean) => void
  onHistory: (direction: 'undo' | 'redo') => void
  onSave: () => void
}

const hydration = Annotation.define<boolean>()

/** CodeMirror contains no local history extension; all undo reaches the document. */
export function FeatureCodeEditor(props: FeatureCodeEditorProps) {
  const container = useRef<HTMLDivElement>(null)
  const editor = useRef<EditorView | null>(null)
  const latest = useRef(props)
  const access = useRef(new Compartment())
  const completion = useRef(new Compartment())
  const languagePhrases = useRef(new Compartment())
  const focusFrom = props.focusRange?.from
  const focusTo = props.focusRange?.to
  const focusRevision = props.focusRange?.revision

  useLayoutEffect(() => {
    latest.current = props
  })

  useLayoutEffect(() => {
    if (!container.current) return
    let composing = false
    let compositionEnd: ReturnType<typeof setTimeout> | undefined
    const view = new EditorView({
      parent: container.current,
      state: EditorState.create({
        doc: latest.current.value,
        extensions: [
          featureLanguage,
          syntaxHighlighting(classHighlighter),
          lineNumbers(),
          highlightActiveLine(),
          bracketMatching(),
          closeBrackets(),
          lintGutter(),
          access.current.of([]),
          completion.current.of([]),
          languagePhrases.current.of([]),
          // Tab remains available for keyboard navigation out of the editor.
          keymap.of([
            ...closeBracketsKeymap,
            ...completionKeymap,
            ...defaultKeymap,
          ]),
          EditorView.updateListener.of((update) => {
            if (
              update.docChanged &&
              !update.transactions.some((transaction) =>
                transaction.annotation(hydration),
              )
            ) {
              latest.current.onChange(update.state.doc.toString())
            }
          }),
          EditorView.domEventHandlers({
            beforeinput(event) {
              if (
                event.inputType !== 'historyUndo' &&
                event.inputType !== 'historyRedo'
              )
                return false
              if (event.isComposing || composing) return false
              event.preventDefault()
              event.stopPropagation()
              latest.current.onHistory(
                event.inputType === 'historyUndo' ? 'undo' : 'redo',
              )
              return true
            },
            keydown(event) {
              if (!(event.ctrlKey || event.metaKey) || event.altKey)
                return false
              const key = event.key.toLowerCase()
              if (key !== 's' && key !== 'z' && key !== 'y') return false
              if (key !== 's' && (event.isComposing || composing)) return false
              event.preventDefault()
              event.stopPropagation()
              if (key === 's') latest.current.onSave()
              else
                latest.current.onHistory(
                  key === 'y' || event.shiftKey ? 'redo' : 'undo',
                )
              return true
            },
          }),
          EditorView.domEventObservers({
            compositionstart() {
              clearTimeout(compositionEnd)
              composing = true
              latest.current.onCompositionChange(true)
            },
            compositionend() {
              clearTimeout(compositionEnd)
              // Let CM consume final DOM mutations, including delayed mobile IME input.
              compositionEnd = setTimeout(() => {
                composing = false
                latest.current.onCompositionChange(false)
              }, 60)
            },
          }),
        ],
      }),
    })
    editor.current = view
    return () => {
      clearTimeout(compositionEnd)
      view.destroy()
      editor.current = null
      if (composing) latest.current.onCompositionChange(false)
    }
  }, [])

  useLayoutEffect(() => {
    editor.current?.dispatch({
      effects: access.current.reconfigure([
        EditorState.readOnly.of(props.readOnly),
        EditorView.editable.of(!props.readOnly),
        EditorView.contentAttributes.of({
          'aria-label': props.label,
          'aria-readonly': String(props.readOnly),
          'aria-multiline': 'true',
          tabindex: '0',
          spellcheck: 'false',
        }),
      ]),
    })
  }, [props.readOnly, props.label])

  useLayoutEffect(() => {
    editor.current?.dispatch({
      effects: completion.current.reconfigure(
        autocompletion({
          override: [
            createFeatureCompletion(
              props.glyphNames,
              props.classNames,
              props.loadGlyphNames,
            ),
          ],
          maxRenderedOptions: 100,
        }),
      ),
    })
  }, [props.glyphNames, props.classNames, props.loadGlyphNames])

  useLayoutEffect(() => {
    editor.current?.dispatch({
      effects: languagePhrases.current.reconfigure(
        EditorState.phrases.of(props.phrases ?? {}),
      ),
    })
  }, [props.phrases])

  useLayoutEffect(() => {
    const view = editor.current
    if (
      !view ||
      view.state.doc.toString() === props.value.replace(/\r\n?/g, '\n')
    )
      return
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: props.value },
      annotations: hydration.of(true),
    })
  }, [props.value])

  useLayoutEffect(() => {
    const view = editor.current
    if (!view) return
    const diagnostics: Diagnostic[] = props.diagnostics.map((diagnostic) => {
      const from = sourceOffsetToEditorOffset(props.value, diagnostic.from)
      return {
        ...diagnostic,
        from,
        to: Math.max(
          from,
          sourceOffsetToEditorOffset(props.value, diagnostic.to),
        ),
      }
    })
    view.dispatch(setDiagnostics(view.state, diagnostics))
  }, [props.diagnostics, props.value])

  useLayoutEffect(() => {
    const view = editor.current
    if (!view || focusFrom === undefined || focusTo === undefined) return
    view.dispatch({
      selection: {
        anchor: sourceOffsetToEditorOffset(latest.current.value, focusFrom),
        head: sourceOffsetToEditorOffset(latest.current.value, focusTo),
      },
      scrollIntoView: true,
    })
    view.focus()
  }, [focusFrom, focusTo, focusRevision])

  return <div ref={container} className={styles.editor} />
}
