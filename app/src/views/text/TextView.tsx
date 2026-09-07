import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { type DocSummary } from '../../api'
import { activeEditor } from '../../editor/activeEditor'
import { reportError } from '../../shell/commands'
import { isEditableTarget } from '../../shell/shortcuts'
import { selectDocView, useDocumentStore } from '../../stores/documentStore'
import { useEditorStore } from '../../stores/editorStore'
import { type Tab, type TextTabState, useUiStore } from '../../stores/uiStore'
import { GlyphView } from '../glyph/GlyphView'
import { TextBar, TEXT_SAMPLE_TEXT } from './TextBar'
import { PreviewStrip } from './PreviewStrip'
import type { TextSampleId } from './presentationTypes'
import type { TextCanvasRun } from './textCanvas'
import { textRunBounds } from './textCanvasBridge'
import {
  previewGlyphIndexes,
  runBox,
  MAX_VISIBLE_RUN_GLYPHS,
} from './textRunGeometry'
import {
  glyphText,
  initialTextState,
  reconcileTextIndex,
  textTitle,
} from './textState'
import { useTextGlyphCells } from './useTextGlyphCells'
import { useTextMetrics } from './useTextMetrics'
import { useTextShape } from './useTextShape'
import { useTextFeedback } from './useTextFeedback'
import { allEncodedText } from './textSamples'
import styles from './TextView.module.css'

/** The shaped Text tab surrounds one persistent glyph editor/controller. */
export function TextView({ summary, tab }: { summary: DocSummary; tab: Tab }) {
  const { t } = useTranslation()
  const view = useDocumentStore(selectDocView(summary.doc))
  const activeGlyph = useEditorStore((state) => state.activeGlyph)
  const tool = useEditorStore((state) => state.tool)
  const previewVisible = useUiStore((state) => state.textPreviewVisible)
  const fallback = useMemo(
    () => initialTextState(view.master ?? summary.masters[0]?.id ?? null),
    [summary.masters, view.master],
  )
  const state = tab.textState ?? fallback
  const [draft, setDraft] = useState(state.text)
  const [committedText, setCommittedText] = useState(state.text)
  if (committedText !== state.text) {
    setCommittedText(state.text)
    setDraft(state.text)
  }
  const inputRef = useRef<HTMLInputElement>(null)
  const latest = useRef({ tab, state })
  useEffect(() => {
    latest.current = { tab, state }
  }, [tab, state])
  const [visible, setVisible] = useState<readonly number[]>([])
  const [entered, setEntered] = useState(false)
  const [dismissed, setDismissed] = useState('')
  const inputRevision = useRef(0)
  const masterBridge = useRef<{
    state: string | null
    view: string | null
  } | null>(null)
  const patch = useCallback(
    (changes: Partial<TextTabState>, gid?: number) => {
      const current = useUiStore
        .getState()
        .tabs.find((candidate) => candidate.id === tab.id)
      if (!current) return
      const next = {
        ...(current.textState ?? latest.current.state),
        ...changes,
      }
      useUiStore.getState().updateTab(tab.id, {
        textState: next,
        label: textTitle(next.text),
        ...(gid === undefined ? {} : { gid }),
      })
    },
    [tab.id],
  )
  const transition = useCallback(
    async (action: () => void) => {
      if (useUiStore.getState().activeTab !== tab.id) return
      const editor = activeEditor()
      await editor?.prepareTransition()
      if (
        useUiStore.getState().activeTab === tab.id &&
        activeEditor() === editor
      )
        action()
    },
    [tab.id],
  )
  const enterText = useCallback(() => {
    void transition(() => {
      patch({ mode: 'text' })
      useEditorStore.getState().setTool('text')
      inputRef.current?.focus({ preventScroll: true })
    }).catch(reportError)
  }, [patch, transition])

  // Lazy migration never replaces user input entered before glyph metadata.
  useEffect(() => {
    if (
      !activeGlyph ||
      activeGlyph.doc !== tab.doc ||
      activeGlyph.gid !== tab.gid
    )
      return
    if (
      !tab.textState ||
      (tab.textState.activeIndex === null && tab.textState.mode === 'edit')
    )
      patch({ text: glyphText(activeGlyph.glyph), activeIndex: 0 })
  }, [activeGlyph, patch, tab.doc, tab.gid, tab.textState])

  useEffect(() => {
    const prior = masterBridge.current
    masterBridge.current = { state: state.master, view: view.master }
    if (!prior || state.master !== prior.state) {
      if (state.master !== view.master)
        useDocumentStore.getState().setMaster(tab.doc, state.master)
    } else if (view.master !== prior.view && state.master !== view.master) {
      void transition(() => patch({ master: view.master })).catch(reportError)
    }
  }, [patch, state.master, tab.doc, transition, view.master])

  const context = useMemo(
    () => ({
      doc: summary.doc,
      generation: view.generation,
      unitsPerEm: summary.units_per_em,
      master: state.master,
      text: state.text,
      features: state.features,
      direction: state.direction,
      script: null,
      language: null,
    }),
    [
      state.direction,
      state.features,
      state.master,
      state.text,
      summary.doc,
      summary.units_per_em,
      view.generation,
    ],
  )
  const shaped = useTextShape(tab.textState ? context : null)
  const glyphs = shaped?.layout.glyphs ?? EMPTY_GLYPHS
  if (!entered && state.mode === 'text' && glyphs.length > 0) setEntered(true)
  useEffect(() => {
    if (import.meta.env.DEV)
      globalThis.__fwTextRun = { glyphs, activeIndex: state.activeIndex }
    return () => {
      if (import.meta.env.DEV) globalThis.__fwTextRun = null
    }
  }, [glyphs, state.activeIndex])
  const prior = useRef<{ glyphs: typeof glyphs; active: number | null }>({
    glyphs,
    active: state.activeIndex,
  })
  useEffect(() => {
    const old = prior.current
    prior.current = { glyphs, active: state.activeIndex }
    if (!shaped || glyphs === old.glyphs || state.mode !== 'edit') return
    const previous = old.active === null ? undefined : old.glyphs[old.active]
    const index = previous
      ? reconcileTextIndex(previous, glyphs)
      : glyphs.findIndex((glyph) => glyph.documentGid === tab.gid)
    if (index === null || index < 0) {
      patch({ activeIndex: null, mode: 'text' })
      return
    }
    const gid = glyphs[index]?.documentGid
    if (index !== state.activeIndex || (gid !== null && gid !== tab.gid))
      patch({ activeIndex: index }, gid ?? undefined)
  }, [glyphs, patch, shaped, state.activeIndex, state.mode, tab.gid])
  const previewIndexes = useMemo(
    () => previewGlyphIndexes(glyphs, summary.units_per_em),
    [glyphs, summary.units_per_em],
  )
  const wanted = useMemo(
    () =>
      [
        ...new Set([
          ...(state.activeIndex === null ? [] : [state.activeIndex]),
          ...visible,
          ...(previewVisible && !state.previewCollapsed ? previewIndexes : []),
        ]),
      ].slice(0, MAX_VISIBLE_RUN_GLYPHS),
    [
      previewIndexes,
      previewVisible,
      state.activeIndex,
      state.previewCollapsed,
      visible,
    ],
  )
  const cells = useTextGlyphCells(
    summary.doc,
    state.master,
    shaped?.context.generation ?? view.generation,
    glyphs,
    wanted,
  )
  const metrics = useTextMetrics(
    state.mode === 'edit',
    t('textView.metricsUndo'),
  )
  const activate = useCallback(
    (index: number) => {
      const selected = glyphs[index]
      if (!selected?.editable || selected.documentGid === null) return
      void transition(() => {
        const current = latest.current.state
        if (
          current.text !== state.text ||
          current.master !== state.master ||
          current.direction !== state.direction ||
          JSON.stringify(current.features) !== JSON.stringify(state.features)
        )
          return
        patch({ activeIndex: index, mode: 'edit' }, selected.documentGid!)
        useEditorStore.getState().setTool('select')
      }).catch(reportError)
    },
    [
      glyphs,
      patch,
      state.text,
      state.master,
      state.direction,
      state.features,
      transition,
    ],
  )
  const active =
    state.activeIndex === null ? undefined : glyphs[state.activeIndex]
  const textRun = useMemo<TextCanvasRun | null>(() => {
    if (!tab.textState) return null
    const run: TextCanvasRun = {
      glyphs,
      cells,
      activeIndex: state.activeIndex,
      activeOrigin: { x: active?.x ?? 0, y: active?.y ?? 0 },
      mode: state.mode,
      unitsPerEm: summary.units_per_em,
      fitBox: runBox(glyphs, new Map(), summary.units_per_em),
      requestedScale: state.size / summary.units_per_em,
      onActivate: activate,
      onExitEdit: enterText,
      onVisibleIndexes: setVisible,
      onMetricKey: metrics.onMetricKey,
    }
    run.fitBox = runBox(glyphs, textRunBounds({ cells }), summary.units_per_em)
    return run
  }, [
    activate,
    active?.x,
    active?.y,
    cells,
    enterText,
    glyphs,
    metrics.onMetricKey,
    state.activeIndex,
    state.mode,
    state.size,
    summary.units_per_em,
    tab.textState,
  ])
  const previewGlyphs = useMemo(
    () =>
      previewIndexes.flatMap((index) => {
        const glyph = glyphs[index]
        const path =
          glyph?.documentGid === null || !glyph
            ? undefined
            : cells.get(glyph.documentGid)?.path
        return glyph && path ? [{ ...glyph, path }] : []
      }),
    [cells, glyphs, previewIndexes],
  )

  const changeText = useCallback(
    (text: string) => {
      const revision = ++inputRevision.current
      setDraft(text)
      void transition(() => {
        if (revision !== inputRevision.current) return
        patch({ text, mode: 'text', activeIndex: null })
      }).catch(reportError)
    },
    [patch, transition],
  )
  useEffect(() => {
    if (tool === 'text' && state.mode !== 'text') enterText()
  }, [enterText, state.mode, tool])
  useEffect(() => {
    const typing = (event: KeyboardEvent) => {
      if (
        useUiStore.getState().modalCount > 0 ||
        isEditableTarget(event.target) ||
        event.ctrlKey ||
        event.metaKey ||
        event.altKey
      )
        return
      if (state.mode !== 'text' || event.key === ' ') return
      if (event.key.length !== 1 && event.key !== 'Backspace') return
      event.preventDefault()
      event.stopImmediatePropagation()
      const next =
        event.key === 'Backspace'
          ? Array.from(latest.current.state.text).slice(0, -1).join('')
          : latest.current.state.text + event.key
      changeText(next)
      inputRef.current?.focus({ preventScroll: true })
    }
    window.addEventListener('keydown', typing, true)
    return () => window.removeEventListener('keydown', typing, true)
  }, [changeText, state.mode])
  const sample = async (id: TextSampleId) => {
    if (id !== 'all-encoded') return changeText(TEXT_SAMPLE_TEXT[id])
    const request = ++inputRevision.current
    const requestedMaster = state.master
    const text = await allEncodedText(
      summary.doc,
      requestedMaster,
      () =>
        request === inputRevision.current &&
        latest.current.state.master === requestedMaster &&
        useUiStore.getState().activeTab === tab.id,
    )
    if (text !== null) changeText(text)
  }

  const status = shaped?.compileFailure
    ? 'compile-failed'
    : !shaped || shaped.stale || shaped.context.generation !== view.generation
      ? 'stale'
      : 'ready'
  useTextFeedback(tab.id, glyphs.length, status === 'stale')
  const statusKey = `${view.generation}:${state.text}:${status}`
  return (
    <div
      className={styles.view}
      data-text-mode={state.mode}
      data-run-count={glyphs.length}
    >
      <TextBar
        text={draft}
        inputRef={inputRef}
        onTextChange={changeText}
        size={state.size}
        onSizeChange={(size) => void transition(() => patch({ size }))}
        master={state.master ?? ''}
        masters={summary.masters.map((master) => ({
          id: master.id,
          name: master.name,
        }))}
        onMasterChange={(master) => void transition(() => patch({ master }))}
        features={state.features}
        onFeaturesChange={(features) =>
          void transition(() => patch({ features }))
        }
        direction={state.direction}
        onDirectionChange={(direction) =>
          void transition(() => patch({ direction }))
        }
        onSample={(id) => void sample(id).catch(reportError)}
      />
      <GlyphView
        summary={summary}
        gid={tab.gid}
        tabIdentity={tab.id}
        textRun={textRun}
        textFitKey={`${tab.id}:${entered ? 'text' : 'glyph'}`}
        externalSettler={metrics.finish}
      />
      {previewVisible && (
        <PreviewStrip
          glyphs={previewGlyphs}
          unitsPerEm={summary.units_per_em}
          status={dismissed === statusKey ? 'ready' : status}
          onDismissStatus={() => setDismissed(statusKey)}
          theme={state.previewTheme}
          onThemeChange={(previewTheme) => patch({ previewTheme })}
          collapsed={state.previewCollapsed}
          onCollapsedChange={(previewCollapsed) => patch({ previewCollapsed })}
        />
      )}
      <span className={styles.srOnly} role="status">
        {t('textView.runCount', { count: glyphs.length })}
      </span>
    </div>
  )
}
const EMPTY_GLYPHS: readonly import('./textLayout').PositionedGlyph[] = []

declare global {
  var __fwTextRun:
    | {
        glyphs: readonly import('./textLayout').PositionedGlyph[]
        activeIndex: number | null
      }
    | null
    | undefined
}
