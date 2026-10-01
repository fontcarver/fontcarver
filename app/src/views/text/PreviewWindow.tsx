import { useMemo, useState } from 'react'
import type { DocSummary } from '../../api'
import { selectDocView, useDocumentStore } from '../../stores/documentStore'
import { type Tab, useUiStore } from '../../stores/uiStore'
import { initialTextState } from './textState'
import { useTextShape } from './useTextShape'
import { useTextRunCells } from './useTextRunCells'
import { PreviewStrip } from './PreviewStrip'
import { previewSettingsOf, previewSettingsPatch } from './previewSettings'
import { usePreviewInstance } from './usePreviewInstance'
import { useTextContext } from './useTextContext'
import type { PositionedGlyph } from './textLayout'

const EMPTY: readonly PositionedGlyph[] = []
/** Independent window host that observes the document without installing a glyph editor. */
export function PreviewWindow({
  summary,
  tab,
}: {
  summary: DocSummary
  tab: Tab
}) {
  const view = useDocumentStore(selectDocView(summary.doc))
  const fallback = useMemo(
    () => initialTextState(summary.masters[0]?.id ?? null),
    [summary.masters],
  )
  const state = tab.textState ?? fallback
  const [dismissed, setDismissed] = useState('')
  const context = useTextContext(summary, state, view.generation)
  const shaped = useTextShape(context)
  const { cells, colors, previewGlyphs } = useTextRunCells(
    summary.doc,
    state.master,
    view.generation,
    summary.units_per_em,
    shaped?.layout.glyphs ?? EMPTY,
    state.activeIndex,
    [],
    true,
  )
  const patch = (changes: Partial<typeof state>) =>
    useUiStore
      .getState()
      .updateTab(tab.id, { textState: { ...state, ...changes } })
  const active =
    state.activeIndex === null ? null : shaped?.layout.glyphs[state.activeIndex]
  const status = shaped?.compileFailure
    ? 'compile-failed'
    : !shaped || shaped.stale
      ? 'stale'
      : 'ready'
  const statusKey = `${context.generation}:${context.text}:${status}`
  const previewSource = usePreviewInstance({
    summary,
    state,
    generation: view.generation,
    documentGlyphs: previewGlyphs,
    documentActiveGlyph:
      active && active.documentGid !== null
        ? {
            ...active,
            path: cells.get(active.documentGid)?.path ?? '',
            color: colors.get(active.documentGid),
          }
        : null,
  })
  return (
    <PreviewStrip
      glyphs={[...previewSource.glyphs]}
      unitsPerEm={summary.units_per_em}
      status={
        dismissed === statusKey ? 'ready' : (previewSource.status ?? status)
      }
      onDismissStatus={() => setDismissed(statusKey)}
      theme={state.previewTheme}
      onThemeChange={(previewTheme) => patch({ previewTheme })}
      collapsed={state.previewCollapsed}
      onCollapsedChange={(previewCollapsed) => patch({ previewCollapsed })}
      height={state.previewHeight ?? 600}
      onHeightChange={(previewHeight) => patch({ previewHeight })}
      settings={previewSettingsOf(state)}
      onSettingsChange={(changes) => patch(previewSettingsPatch(changes))}
      instances={previewSource.instances}
      activeGlyph={previewSource.activeGlyph}
    />
  )
}
