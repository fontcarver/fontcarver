/**
 * The Layer menu and the two Glyph-menu items that act on the open glyph.
 *
 * They are the Layers panel's actions and the Components panel's "decompose
 * all", reachable from the menu bar and the palette so nothing is more than
 * one shortcut or two clicks away (design spec §13). Each one commits exactly
 * the command the panel button commits.
 */

import { activeEditor } from '../../editor/activeEditor'
import { applyCommand } from '../../editor/editSession'
import { i18n } from '../../i18n'
import { registerCommands } from '../../shell/commands'
import { useEditorStore } from '../../stores/editorStore'
import { openGlyphTab } from '../../shell/tabActions'
import { call } from '../../api'
import { reportError } from '../../shell/commands'

import { commitField } from './commitField'
import { backgroundLayer } from './glyphScene'
import { pathContext } from './pathCommands'
import {
  addLayerCommand,
  copyLayerCommand,
  decomposeComponentsCommand,
  duplicateGlyphCommand,
  duplicateName,
  swapWithBackgroundCommand,
} from './panels/panelCommands'

/** The glyph the Layer menu acts on, with everything its commands name. */
function layerScope() {
  const active = useEditorStore.getState().activeGlyph
  const outline = active?.outline
  if (!active || !outline || active.master === null) return null
  return {
    active,
    outline,
    master: active.master,
    target: {
      doc: active.doc,
      glyph: active.glyph.name,
      layer: outline.layer,
    },
  }
}

function hasMasterLayer(): boolean {
  return layerScope() !== null
}

/** Registers the Layer menu and the glyph-scoped Glyph-menu items. */
export function registerLayerCommands(): void {
  registerCommands([
    {
      id: 'layer.addBackground',
      title: 'cmd.layer.addBackground',
      menu: 'layer',
      group: 'background',
      when: () => {
        const scope = layerScope()
        return (
          scope !== null &&
          backgroundLayer(scope.active.glyph, scope.master) === null
        )
      },
      disabledReason: 'cmd.reason.noGlyphLayer',
      run: () => {
        const scope = layerScope()
        if (!scope) return
        commitField(
          scope.target,
          addLayerCommand(
            scope.target.glyph,
            { Background: scope.master },
            i18n.t('glyphInspector.layerKind.background'),
            scope.outline.width,
          ),
          i18n.t('editor.undo.addLayer'),
          scope.active.reload,
        )
      },
    },
    {
      id: 'layer.copyToBackground',
      title: 'cmd.layer.copyToBackground',
      menu: 'layer',
      group: 'background',
      when: hasMasterLayer,
      disabledReason: 'cmd.reason.noGlyphLayer',
      run: () => {
        const scope = layerScope()
        if (!scope) return
        commitField(
          scope.target,
          copyLayerCommand(scope.target.glyph, scope.outline.layer, {
            Background: scope.master,
          }),
          i18n.t('editor.undo.copyToBackground'),
          scope.active.reload,
        )
      },
    },
    {
      id: 'layer.swapWithBackground',
      title: 'cmd.layer.swapWithBackground',
      menu: 'layer',
      group: 'background',
      when: () => {
        const scope = layerScope()
        return (
          scope !== null &&
          backgroundLayer(scope.active.glyph, scope.master) !== null
        )
      },
      disabledReason: 'cmd.reason.noGlyphLayer',
      run: () => {
        const scope = layerScope()
        if (!scope) return
        commitField(
          scope.target,
          swapWithBackgroundCommand(scope.target.glyph, scope.master),
          i18n.t('editor.undo.swapWithBackground'),
          scope.active.reload,
        )
      },
    },
    {
      id: 'layer.addBackup',
      title: 'cmd.layer.addBackup',
      menu: 'layer',
      group: 'backup',
      when: hasMasterLayer,
      disabledReason: 'cmd.reason.noGlyphLayer',
      run: () => {
        const scope = layerScope()
        if (!scope) return
        commitField(
          scope.target,
          copyLayerCommand(scope.target.glyph, scope.outline.layer, {
            Backup: {
              of: scope.master,
              time: Math.floor(Date.now() / 1000),
            },
          }),
          i18n.t('editor.undo.addBackup'),
          scope.active.reload,
        )
      },
    },
    {
      id: 'glyph.decomposeAll',
      title: 'cmd.glyph.decomposeAll',
      menu: 'glyph',
      group: 'glyph',
      when: () => {
        const context = pathContext()
        const active = useEditorStore.getState().activeGlyph
        return context !== null && (active?.outline?.components.length ?? 0) > 0
      },
      disabledReason: 'cmd.reason.noGlyphLayer',
      run: () => {
        const context = pathContext()
        const active = useEditorStore.getState().activeGlyph
        const components = active?.outline?.components ?? []
        if (!context || components.length === 0) return
        commitField(
          context.target,
          decomposeComponentsCommand(
            context.target,
            components.map((_, index) => index),
          ),
          i18n.t('editor.undo.decompose'),
          context.reload,
        )
      },
    },
    {
      id: 'glyph.duplicate',
      title: 'cmd.glyph.duplicate',
      menu: 'glyph',
      group: 'glyph',
      when: () => useEditorStore.getState().activeGlyph !== null,
      disabledReason: 'cmd.reason.noGlyphTab',
      run: () => {
        const active = useEditorStore.getState().activeGlyph
        if (!active) return
        const source = active.glyph.name
        // The copy needs a name no glyph holds; the kernel is the authority on
        // what exists, so the candidates are checked against a search of the
        // stem rather than against whatever the interface has cached.
        void Promise.resolve(activeEditor()?.settle())
          .then(() =>
            call('glyphs.search', {
              doc: active.doc,
              query: source,
              limit: 64,
            }),
          )
          .then((cells) => {
            const taken = new Set(cells.map((cell) => cell.name))
            const name = duplicateName(source, taken)
            return applyCommand(
              active.doc,
              duplicateGlyphCommand(source, name),
              i18n.t('editor.undo.duplicateGlyph'),
            ).then(() => name)
          })
          .then((name) =>
            call('glyphs.search', {
              doc: active.doc,
              query: name,
              limit: 8,
            }).then((cells) => {
              const cell = cells.find((candidate) => candidate.name === name)
              if (cell) openGlyphTab(active.doc, cell.gid, cell.name)
            }),
          )
          .catch(reportError)
          .finally(() => {
            active.reload()
          })
      },
    },
  ])
}
