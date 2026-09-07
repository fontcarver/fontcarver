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
import { activeEditor } from '../../editor/activeEditor'
import { selectionCounts } from '../../editor/selection'
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

/** Whether the canvas of a glyph tab currently owns the keyboard. */
function canvasHasFocus(): boolean {
  return (
    onGlyphTab() &&
    globalThis.document?.activeElement instanceof HTMLCanvasElement
  )
}

/** Whether an editor is mounted and something is selected in it. */
function hasSelection(): boolean {
  const editor = activeEditor()
  return onGlyphTab() && editor !== null && editor.currentSelection.size > 0
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
    {
      // The Font view owns `edit.selectAll` (every glyph in the font); this is
      // the Edit tab's own "every point of this layer". Sharing the id would
      // let whichever view registers last replace the other.
      id: 'edit.selectAllPoints',
      title: 'cmd.edit.selectAllPoints',
      menu: 'edit',
      group: 'selection',
      shortcut: 'Mod+A',
      // Gated on the tab kind alone: Mod+A must belong to exactly one of the
      // two select-all commands at any moment, and "which tab is open" is what
      // decides that. A tab whose editor is still loading simply does nothing.
      when: onGlyphTab,
      disabledReason: 'cmd.reason.noGlyphTab',
      run: () => activeEditor()?.selectAll(),
    },
    {
      id: 'edit.deselectPoints',
      title: 'cmd.edit.deselectPoints',
      menu: 'edit',
      group: 'selection',
      when: hasSelection,
      disabledReason: 'cmd.reason.noPointsSelected',
      run: () => activeEditor()?.escape(),
    },
    {
      id: 'edit.deletePoints',
      title: 'cmd.edit.deletePoints',
      menu: 'edit',
      group: 'selection',
      shortcut: 'Delete',
      when: hasSelection,
      disabledReason: 'cmd.reason.noPointsSelected',
      run: () => activeEditor()?.deleteSelection(false),
    },
    {
      id: 'edit.deletePointsBreak',
      title: 'cmd.edit.deletePointsBreak',
      menu: 'edit',
      group: 'selection',
      shortcut: 'Shift+Delete',
      when: hasSelection,
      disabledReason: 'cmd.reason.noPointsSelected',
      run: () => activeEditor()?.deleteSelection(true),
    },
    {
      id: 'path.toggleSmooth',
      title: 'cmd.path.toggleSmooth',
      menu: 'path',
      group: 'nodes',
      shortcut: 'Enter',
      when: () => {
        const editor = activeEditor()
        if (!editor || !canvasHasFocus()) return false
        // A pen path in progress owns Enter: it finishes the path, and the
        // shortcut layer must leave the key alone rather than claiming it for
        // whatever node happens to be selected underneath.
        if (editor.claimsKey('Enter')) return false
        return selectionCounts(editor.currentSelection).nodes > 0
      },
      disabledReason: 'cmd.reason.noPointsSelected',
      run: () => activeEditor()?.toggleSmooth(false),
    },
    {
      id: 'view.toggleSnapping',
      title: 'cmd.view.toggleSnapping',
      menu: 'view',
      group: 'panels',
      when: onGlyphTab,
      disabledReason: 'cmd.reason.noGlyphTab',
      run: () => {
        const store = useEditorStore.getState()
        store.setSnapping(!store.snapping)
      },
    },
    {
      id: 'view.toggleInfoBox',
      title: 'cmd.view.toggleInfoBox',
      menu: 'view',
      group: 'panels',
      shortcut: 'Mod+Shift+I',
      when: onGlyphTab,
      disabledReason: 'cmd.reason.noGlyphTab',
      run: () => useEditorStore.getState().toggleInfoBox(),
    },
    overlayCommand('filled'),
    overlayCommand('points'),
    overlayCommand('metrics'),
    overlayCommand('anchors'),
    overlayCommand('guides'),
    overlayCommand('rulers'),
    overlayCommand('background'),
    overlayCommand('otherMasters'),
  ])
}
