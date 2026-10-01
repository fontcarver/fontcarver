import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
} from 'react'
import { useTranslation } from 'react-i18next'
import { type DocSummary } from '../../api'
import { activeEditor } from '../../editor/activeEditor'
import { reportError } from '../../shell/commands'
import { selectDocView, useDocumentStore } from '../../stores/documentStore'
import { useEditorStore } from '../../stores/editorStore'
import { type Tab, type TextTabState, useUiStore } from '../../stores/uiStore'
import { GlyphView } from '../glyph/GlyphView'
import { TEXT_SAMPLE_TEXT } from './TextBar'
import { TextWorkspaceControls } from './TextWorkspaceControls'
import { PreviewStrip } from './PreviewStrip'
import type { TextSampleId } from './presentationTypes'
import type { TextCanvasRun } from './textCanvas'
import { textRunBounds } from './textCanvasBridge'
import { runBox } from './textRunGeometry'
import {
  glyphText,
  initialTextState,
  reconcileTextIndex,
  textTitle,
} from './textState'
import { useTextRunCells } from './useTextRunCells'
import { useTextKerningControls } from './useTextKerningControls'
import { useTextMetrics } from './useTextMetrics'
import { useTextShape } from './useTextShape'
import { useTextFeedback } from './useTextFeedback'
import { useTextTyping } from './useTextTyping'
import { allEncodedText } from './textSamples'
import type { ProofEntry } from './proofLibrary'
import { previewSettingsOf, previewSettingsPatch } from './previewSettings'
import { canStepFromSurface, proofStepDirection } from './proofStepping'
import { usePreviewInstance } from './usePreviewInstance'
import { useTextContext } from './useTextContext'
import { isEditableTarget, isInteractiveTarget } from '../../shell/shortcuts'
import { detachPanel } from '../../shell/windowActions'
import { useKerningGhosts } from './useKerningGhosts'
import { useSpacingOverlay } from './useSpacingOverlay'
import { useKerningExceptions } from './useKerningExceptions'
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
  const kerningMode = state.kerningMode ?? 'on'
  const context = useTextContext(summary, state, view.generation)
  const shapingFeatures = context.features
  const [draft, setDraft] = useState(state.text)
  const [committedText, setCommittedText] = useState(state.text)
  if (committedText !== state.text) {
    setCommittedText(state.text)
    setDraft(state.text)
  }
  const inputRef = useRef<HTMLInputElement>(null)
  const latest = useRef({ tab, state })
  useLayoutEffect(() => {
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

  const shaped = useTextShape(tab.textState ? context : null)
  const glyphs = shaped?.layout.glyphs ?? EMPTY_GLYPHS
  if (!entered && state.mode === 'text' && glyphs.length > 0) setEntered(true)
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
  const { cells, colors, previewGlyphs } = useTextRunCells(
    summary.doc,
    state.master,
    shaped?.context.generation ?? view.generation,
    summary.units_per_em,
    glyphs,
    state.activeIndex,
    visible,
    previewVisible && !state.previewCollapsed,
  )
  const metrics = useTextMetrics(
    state.mode === 'edit' && kerningMode !== 'locked',
    t('textView.metricsUndo'),
    state.spacingSide ?? 'lsb',
  )
  const pending =
    !shaped || shaped.stale || shaped.context.generation !== view.generation
  const kerning = useTextKerningControls({
    doc: summary.doc,
    master: state.master,
    generation: view.generation,
    text: state.text,
    direction: state.direction,
    features: shapingFeatures,
    kerningMode,
    glyphs,
    pending,
    transition,
  })
  // Spacing panel and its canvas overlays share one bounded metrics request.
  const spacing = useSpacingOverlay({
    tab: tab.id,
    doc: summary.doc,
    master: state.master,
    generation: view.generation,
    unitsPerEm: summary.units_per_em,
    glyphs,
  })
  const kerningExceptions = useKerningExceptions({
    doc: summary.doc,
    master: state.master,
    generation: view.generation,
    gaps: kerning.gaps,
    vertical: state.direction === 'ttb',
    enabled: !pending && kerningMode !== 'off',
  })
  const ghosts = useKerningGhosts({
    doc: summary.doc,
    master: state.master,
    generation: view.generation,
    model: kerning.panel,
    glyphs,
    selectedGap: kerning.selectedGap,
    pending,
    direction: state.direction,
  })
  const finishMetrics = metrics.finish
  const finishKerning = kerning.finish
  const finishEdits = useCallback(async () => {
    await Promise.all([finishMetrics(), finishKerning()])
  }, [finishMetrics, finishKerning])
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
          current.script !== state.script ||
          current.language !== state.language ||
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
      state.script,
      state.language,
      transition,
    ],
  )
  const active =
    state.activeIndex === null ? undefined : glyphs[state.activeIndex]
  useEffect(() => {
    if (import.meta.env.DEV)
      globalThis.__fwTextRun = {
        glyphs,
        activeIndex: state.activeIndex,
        selectedGap: kerning.selectedGap,
        activeOrigin: { x: active?.x ?? 0, y: active?.y ?? 0 },
      }
    return () => {
      if (import.meta.env.DEV) globalThis.__fwTextRun = null
    }
  }, [glyphs, state.activeIndex, kerning.selectedGap, active?.x, active?.y])
  const textRun = useMemo<TextCanvasRun | null>(() => {
    if (!tab.textState) return null
    const run: TextCanvasRun = {
      glyphs,
      cells,
      colors,
      activeIndex: state.activeIndex,
      activeOrigin: { x: active?.x ?? 0, y: active?.y ?? 0 },
      mode: state.mode,
      unitsPerEm: summary.units_per_em,
      fitBox: runBox(glyphs, new Map(), summary.units_per_em),
      requestedScale: state.size / summary.units_per_em,
      gaps: pending || kerningMode === 'off' ? [] : kerning.gaps,
      kerningGhosts: ghosts.outlines,
      selectedKerningValue: ghosts.value,
      selectedKerningException: kerning.panel.resolution?.exception ?? false,
      kerningExceptions,
      beam: spacing.beam,
      cutOff: spacing.cutOff,
      onBeamGrab: spacing.onBeamGrab,
      onBeamDrag: spacing.onBeamDrag,
      onBeamRelease: spacing.onBeamRelease,
      selectedGap: kerning.selectedGap,
      onSelectGap: kerning.selectGap,
      onKerningKey: kerning.onKerningKey,
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
    colors,
    enterText,
    glyphs,
    metrics.onMetricKey,
    ghosts.outlines,
    ghosts.value,
    kerningMode,
    kerning.gaps,
    kerning.panel.resolution?.exception,
    kerningExceptions,
    spacing.beam,
    spacing.cutOff,
    spacing.onBeamGrab,
    spacing.onBeamDrag,
    spacing.onBeamRelease,
    kerning.selectedGap,
    kerning.selectGap,
    kerning.onKerningKey,
    pending,
    state.activeIndex,
    state.mode,
    state.size,
    summary.units_per_em,
    tab.textState,
  ])

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
  useTextTyping(
    state.text,
    state.mode,
    changeText,
    inputRef,
    kerningMode === 'locked' && kerning.selectedGap !== null,
  )
  const applyProof = useCallback(
    (entry: ProofEntry) => {
      const revision = ++inputRevision.current
      setDraft(entry.text)
      void transition(() => {
        if (revision !== inputRevision.current) return
        patch({
          text: entry.text,
          script: entry.script || null,
          language: entry.language || null,
          direction: entry.direction,
          activeIndex: null,
          mode: 'text',
          proofId: entry.id,
        })
      }).catch(reportError)
    },
    [patch, transition],
  )
  // The proof library owns the visible, filtered list; it registers its stepper
  // here so bare Up/Down also work while spacing, where the canvas has focus.
  const stepProof = useRef<((direction: -1 | 1) => void) | null>(null)
  const onSurfaceKeyDown = useCallback(
    (event: ReactKeyboardEvent<HTMLDivElement>) => {
      const direction = proofStepDirection(event)
      if (direction === null) return
      if (
        !canStepFromSurface({
          hasSelection: useEditorStore.getState().selection.size > 0,
          editableTarget: isEditableTarget(event.target),
          interactiveTarget: isInteractiveTarget(event.target),
        })
      )
        return
      const step = stepProof.current
      if (!step) return
      event.preventDefault()
      event.stopPropagation()
      step(direction)
    },
    [],
  )
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
  const documentActiveGlyph =
    active && active.documentGid !== null
      ? {
          ...active,
          path: cells.get(active.documentGid)?.path ?? '',
          color: colors.get(active.documentGid),
        }
      : null
  const previewSource = usePreviewInstance({
    summary,
    state,
    generation: view.generation,
    documentGlyphs: previewGlyphs,
    documentActiveGlyph,
  })
  return (
    <div
      className={styles.view}
      data-text-mode={state.mode}
      data-run-count={glyphs.length}
      // A run count of zero means "nothing shaped yet" while the kernel is
      // still compiling and "this text has no glyphs" once it is not; the two
      // must be distinguishable to a reader, a test and a screen reader.
      data-text-shaping={status === 'stale' ? 'true' : 'false'}
      onKeyDownCapture={onSurfaceKeyDown}
    >
      <TextWorkspaceControls
        summary={summary}
        state={state}
        context={context}
        draft={draft}
        inputRef={inputRef}
        glyphs={glyphs}
        visible={visible}
        pending={pending}
        kerning={kerning}
        ghosts={ghosts}
        spacing={spacing}
        onTextChange={changeText}
        onChange={(changes) =>
          void transition(() => patch(changes)).catch(reportError)
        }
        onSample={(id) => void sample(id).catch(reportError)}
        onProofSelect={applyProof}
        proofId={state.proofId ?? null}
        stepRef={stepProof}
      />
      <GlyphView
        summary={summary}
        gid={tab.gid}
        tabIdentity={tab.id}
        textRun={textRun}
        textFitKey={`${tab.id}:${entered ? 'text' : 'glyph'}`}
        externalSettler={finishEdits}
      />
      {previewVisible && (
        <PreviewStrip
          glyphs={[...previewSource.glyphs]}
          activeGlyph={previewSource.activeGlyph}
          height={state.previewHeight}
          onHeightChange={(previewHeight) => patch({ previewHeight })}
          onDetach={() => void detachPanel('preview').catch(reportError)}
          unitsPerEm={summary.units_per_em}
          status={
            dismissed === statusKey ? 'ready' : (previewSource.status ?? status)
          }
          onDismissStatus={() => setDismissed(statusKey)}
          theme={state.previewTheme}
          onThemeChange={(previewTheme) => patch({ previewTheme })}
          collapsed={state.previewCollapsed}
          onCollapsedChange={(previewCollapsed) => patch({ previewCollapsed })}
          settings={previewSettingsOf(state)}
          onSettingsChange={(changes) => patch(previewSettingsPatch(changes))}
          instances={previewSource.instances}
        />
      )}
      <span className={styles.srOnly} role="status">
        {t('textView.runCount', { count: glyphs.length })}
      </span>
    </div>
  )
}
const EMPTY_GLYPHS: readonly import('./textLayout').PositionedGlyph[] = []
