/**
 * Commands owned by the glyph view.
 *
 * Zooming and stepping through glyphs need the live canvas, which is not
 * something a command can reach through a store: the viewport is a mutable
 * per-frame object. The mounted view therefore registers a small handle here,
 * and the commands drive that. Everything else — the overlay toggles — is
 * ordinary store state.
 */

import type { DocId, GlyphId } from '../../api'
import { registerCommands, type Command } from '../../shell/commands'
import { isEditableTarget, isInteractiveTarget } from '../../shell/shortcuts'
import { useEditorStore } from '../../stores/editorStore'
import { selectActiveTab, useUiStore } from '../../stores/uiStore'

import type { GlyphOverlays } from './renderGlyph'

/** What a mounted glyph view exposes to the command registry. */
export interface GlyphViewHandle {
  doc: DocId
  gid: GlyphId
  /** Frames the glyph in the canvas. */
  fit: () => void
  /** Multiplies the zoom, anchored on the centre of the canvas. */
  zoomBy: (factor: number) => void
  /** Opens the previous or next glyph of the font view's current order. */
  step: (direction: 1 | -1) => void
}

let active: GlyphViewHandle | null = null

/** Registers the mounted glyph view; the disposer only clears its own handle. */
export function registerGlyphView(handle: GlyphViewHandle): () => void {
  active = handle
  return () => {
    if (active === handle) active = null
  }
}

/** The mounted glyph view, when the active tab is a glyph tab. */
export function activeGlyphView(): GlyphViewHandle | null {
  const tab = selectActiveTab(useUiStore.getState())
  if (tab?.kind !== 'glyph') return null
  if (!active || active.doc !== tab.doc || active.gid !== tab.gid) return null
  return active
}

/** Zoom step applied by the zoom-in and zoom-out commands. */
export const ZOOM_STEP = 1.25

/**
 * Zooms the glyph view when one is active. The shell's zoom commands call this
 * first so `Mod+=` means "zoom the canvas" on a glyph tab and "larger cells" on
 * a font tab, rather than needing two competing bindings.
 */
export function zoomActiveGlyphView(direction: 1 | -1): boolean {
  const view = activeGlyphView()
  if (!view) return false
  view.zoomBy(direction > 0 ? ZOOM_STEP : 1 / ZOOM_STEP)
  return true
}

function onGlyphTab(): boolean {
  return selectActiveTab(useUiStore.getState())?.kind === 'glyph'
}

function canToggleFill(): boolean {
  // The focused element decides ownership of Space: only when the glyph view
  // holds focus on a plain surface (the canvas, or a non-interactive area)
  // does Space toggle the fill. A focused button, link, select or other
  // interactive control keeps Space for its own native activation, even inside
  // a glyph tab.
  const active = globalThis.document?.activeElement ?? null
  return (
    onGlyphTab() && !isEditableTarget(active) && !isInteractiveTarget(active)
  )
}

function overlayCommand(key: keyof GlyphOverlays): Command {
  return {
    id: `view.glyph.${key}`,
    title: `cmd.view.glyph.${key}`,
    menu: 'view',
    group: 'glyphOverlays',
    when: onGlyphTab,
    disabledReason: 'cmd.reason.noGlyphTab',
    run: () => useEditorStore.getState().toggleOverlay(key),
  }
}

/** Registers every command of the glyph view. Safe to call more than once. */
export function registerGlyphViewCommands(): void {
  registerCommands([
    {
      id: 'view.zoomFit',
      title: 'cmd.view.zoomFit',
      menu: 'view',
      group: 'zoom',
      shortcut: 'Mod+0',
      when: () => activeGlyphView() !== null,
      disabledReason: 'cmd.reason.noGlyphTab',
      run: () => activeGlyphView()?.fit(),
    },
    {
      id: 'glyph.previous',
      title: 'cmd.glyph.previous',
      menu: 'glyph',
      group: 'navigate',
      shortcut: 'Mod+[',
      when: () => activeGlyphView() !== null,
      disabledReason: 'cmd.reason.noGlyphTab',
      run: () => activeGlyphView()?.step(-1),
    },
    {
      id: 'glyph.next',
      title: 'cmd.glyph.next',
      menu: 'glyph',
      group: 'navigate',
      shortcut: 'Mod+]',
      when: () => activeGlyphView() !== null,
      disabledReason: 'cmd.reason.noGlyphTab',
      run: () => activeGlyphView()?.step(1),
    },
    {
      id: 'view.toggleFill',
      title: 'cmd.view.glyph.filled',
      shortcut: 'Space',
      when: canToggleFill,
      disabledReason: 'cmd.reason.noGlyphTab',
      run: () => {
        // A focused glyph canvas owns Space until keyup so it can distinguish
        // a tap from a Space-drag. The global command covers a newly opened
        // tab before any explicit click, and focus elsewhere in the view.
        if (globalThis.document?.activeElement instanceof HTMLCanvasElement)
          return
        useEditorStore.getState().toggleOverlay('filled')
      },
    },
    overlayCommand('filled'),
    overlayCommand('points'),
    overlayCommand('metrics'),
    overlayCommand('anchors'),
    overlayCommand('guides'),
    overlayCommand('rulers'),
  ])
}
