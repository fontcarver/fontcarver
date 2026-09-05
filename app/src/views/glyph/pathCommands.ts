/**
 * The Path menu (design spec §11).
 *
 * `PathOp` is a whole-layer operation in the kernel, so overlap removal,
 * extrema, simplification, rounding and stroke expansion always act on the
 * layer; the four contour-level operations (reverse, start point, join, break)
 * take their targets from the canvas selection. Everything goes through
 * `commitField`, which closes any open coalesced edit first and reconciles the
 * view afterwards, so a menu item behaves exactly like a field commit.
 */

import type { Command, Contour } from '../../api'
import type { LayerTarget } from '../../editor/pointCommands'
import type { Selection } from '../../editor/selection'
import {
  registerCommands,
  type Command as MenuCommand,
} from '../../shell/commands'
import { i18n } from '../../i18n'
import { useEditorStore } from '../../stores/editorStore'
import { usePrefsStore } from '../../stores/prefsStore'
import { useUiStore } from '../../stores/uiStore'

import { commitField } from './commitField'
import {
  breakCommand,
  breakPoint,
  DEFAULT_QUADRATIC_ERROR,
  joinCommand,
  joinEnds,
  onlyNode,
  pathOpCommand,
  reverseCommand,
  scopedContours,
  setStartPointCommand,
  tidyCommand,
} from './pathOps'

/** The layer a Path-menu item acts on, with the selection scoping it. */
export interface PathContext {
  target: LayerTarget
  contours: readonly Contour[]
  selection: Selection
  reload: () => void
}

/** The edited layer of the active Edit tab, or `null`. */
export function pathContext(): PathContext | null {
  const state = useEditorStore.getState()
  const active = state.activeGlyph
  const outline = active?.outline
  if (!active || !outline) return null
  return {
    target: {
      doc: active.doc,
      glyph: active.glyph.name,
      layer: outline.layer,
    },
    contours: outline.contours,
    // The canvas publishes its selection into the store, so a menu item reads
    // it there rather than reaching for the mounted controller: the two are
    // the same value, and the store one is available to a palette invocation
    // that runs while the canvas does not hold focus.
    selection: state.selection,
    reload: active.reload,
  }
}

function hasLayer(): boolean {
  return pathContext() !== null
}

/** Applies one command against the edited layer, as its own undo entry. */
export function runOnLayer(command: Command, label: string): void {
  const context = pathContext()
  if (!context) return
  commitField(context.target, command, label, context.reload)
}

/** A Path-menu item that applies one whole-layer operation. */
function opCommand(
  id: string,
  op: Parameters<typeof pathOpCommand>[1],
  labelKey: string,
  shortcut?: string,
): MenuCommand {
  return {
    id: `path.${id}`,
    title: `cmd.path.${id}`,
    menu: 'path',
    group: 'geometry',
    ...(shortcut ? { shortcut } : {}),
    when: hasLayer,
    disabledReason: 'cmd.reason.noGlyphLayer',
    run: () => {
      const context = pathContext()
      if (!context) return
      commitField(
        context.target,
        pathOpCommand(context.target, op),
        i18n.t(labelKey),
        context.reload,
      )
    },
  }
}

/** Registers every Path-menu command. Safe to call more than once. */
export function registerPathCommands(): void {
  registerCommands([
    opCommand(
      'removeOverlap',
      'RemoveOverlap',
      'editor.undo.removeOverlap',
      'Mod+Shift+O',
    ),
    opCommand(
      'correctDirection',
      'CorrectDirection',
      'editor.undo.correctDirection',
      'Mod+Shift+R',
    ),
    opCommand(
      'addExtrema',
      'AddExtrema',
      'editor.undo.addExtrema',
      'Mod+Shift+E',
    ),
    opCommand('addInflections', 'AddInflections', 'editor.undo.addInflections'),
    {
      id: 'path.simplify',
      title: 'cmd.path.simplify',
      menu: 'path',
      group: 'geometry',
      shortcut: 'Mod+Shift+M',
      when: hasLayer,
      disabledReason: 'cmd.reason.noGlyphLayer',
      run: () => useUiStore.getState().setDialog({ kind: 'simplify' }),
    },
    {
      id: 'path.roundToGrid',
      title: 'cmd.path.roundToGrid',
      menu: 'path',
      group: 'geometry',
      when: hasLayer,
      disabledReason: 'cmd.reason.noGlyphLayer',
      run: () => {
        const context = pathContext()
        if (!context) return
        // A grid of zero turns the editing grid off; rounding still has to
        // round to something, and whole units is what "off" means here.
        const grid = usePrefsStore.getState().prefs.grid_size || 1
        commitField(
          context.target,
          pathOpCommand(context.target, { RoundToGrid: grid }),
          i18n.t('editor.undo.roundToGrid'),
          context.reload,
        )
      },
    },
    {
      id: 'path.tidy',
      title: 'cmd.path.tidy',
      menu: 'path',
      group: 'geometry',
      shortcut: 'Mod+Shift+T',
      when: hasLayer,
      disabledReason: 'cmd.reason.noGlyphLayer',
      run: () => {
        const context = pathContext()
        if (!context) return
        commitField(
          context.target,
          tidyCommand(context.target),
          i18n.t('editor.undo.tidy'),
          context.reload,
        )
      },
    },
    {
      id: 'path.expandStroke',
      title: 'cmd.path.expandStroke',
      menu: 'path',
      group: 'geometry',
      when: hasLayer,
      disabledReason: 'cmd.reason.noGlyphLayer',
      run: () => useUiStore.getState().setDialog({ kind: 'expandStroke' }),
    },
    opCommand(
      'convertCubic',
      { ConvertCurves: 'Cubic' },
      'editor.undo.convertCubic',
    ),
    opCommand(
      'convertQuadratic',
      { ConvertCurves: { Quadratic: { max_err: DEFAULT_QUADRATIC_ERROR } } },
      'editor.undo.convertQuadratic',
    ),
    {
      id: 'path.reverse',
      title: 'cmd.path.reverse',
      menu: 'path',
      group: 'contours',
      when: hasLayer,
      disabledReason: 'cmd.reason.noGlyphLayer',
      run: () => {
        const context = pathContext()
        if (!context) return
        commitField(
          context.target,
          reverseCommand(
            context.target,
            scopedContours(context.contours, context.selection),
          ),
          i18n.t('editor.undo.reverseContours'),
          context.reload,
        )
      },
    },
    {
      id: 'path.setStartPoint',
      title: 'cmd.path.setStartPoint',
      menu: 'path',
      group: 'contours',
      when: () => {
        const context = pathContext()
        return context !== null && onlyNode(context.selection) !== null
      },
      disabledReason: 'cmd.reason.noStartPoint',
      run: () => {
        const context = pathContext()
        const node = context && onlyNode(context.selection)
        if (!context || !node) return
        commitField(
          context.target,
          setStartPointCommand(context.target, node),
          i18n.t('editor.undo.setStartPoint'),
          context.reload,
        )
      },
    },
    {
      id: 'path.join',
      title: 'cmd.path.join',
      menu: 'path',
      group: 'contours',
      shortcut: 'Mod+J',
      when: () => {
        const context = pathContext()
        return (
          context !== null &&
          joinEnds(context.contours, context.selection) !== null
        )
      },
      disabledReason: 'cmd.reason.noJoin',
      run: () => {
        const context = pathContext()
        const ends = context && joinEnds(context.contours, context.selection)
        if (!context || !ends) return
        commitField(
          context.target,
          joinCommand(context.target, ends.a, ends.b),
          i18n.t('editor.undo.joinContours'),
          context.reload,
        )
      },
    },
    {
      id: 'path.break',
      title: 'cmd.path.break',
      menu: 'path',
      group: 'contours',
      when: () => {
        const context = pathContext()
        return (
          context !== null &&
          breakPoint(context.contours, context.selection) !== null
        )
      },
      disabledReason: 'cmd.reason.noBreak',
      run: () => {
        const context = pathContext()
        const node = context && breakPoint(context.contours, context.selection)
        if (!context || !node) return
        commitField(
          context.target,
          breakCommand(context.target, node),
          i18n.t('editor.undo.breakContour'),
          context.reload,
        )
      },
    },
  ])
}
