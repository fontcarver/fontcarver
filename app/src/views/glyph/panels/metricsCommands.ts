/**
 * Menu, palette and keyboard access to metrics synchronisation.
 *
 * The Metrics panel already has both update buttons, but the requirement is a
 * command: `Glyph → Update Metrics` for the current master and an alternate for
 * every master. Both act on the glyph being edited, or — from the Font tab — on
 * the whole selection as one undo entry, which is how a font is resynchronised
 * after a spacing pass.
 *
 * Glyphs binds these to ⌘⌃M and ⌘⌥⌃M. Our portable shortcut grammar has no
 * separate Control modifier (Control *is* Mod on Windows and Linux), so the
 * bindings are Mod+Alt+M and Mod+Alt+Shift+M, both remappable.
 */

import { call, type Command, type DocId, type GlyphId } from '../../../api'
import { activeEditor } from '../../../editor/activeEditor'
import {
  holdDocumentEdits,
  pendingDocumentEdits,
} from '../../../editor/editSession'
import { i18n } from '../../../i18n'
import { reportError } from '../../../shell/commands'
import {
  registerCommands,
  type Command as ShellCommand,
} from '../../../shell/commands'
import { useDocumentStore } from '../../../stores/documentStore'
import { useEditorStore } from '../../../stores/editorStore'
import { selectActiveTab, useUiStore } from '../../../stores/uiStore'
import { glyphNamesOf } from '../../font/glyphNames'
import { spacingLockedForDocument } from '../../text/kerningModes'

/** Largest selection one synchronisation command will batch. */
const MAX_SELECTION = 512

interface MetricsTarget {
  doc: DocId
  /** Glyph names to synchronise, in selection order. */
  glyphs: string[]
  /** Master of the current view, used for the current-master variant. */
  master: string | null
  /** Refetches the edited layer once the command lands. */
  reload?: () => void
}

/**
 * What a synchronisation command acts on.
 *
 * The edited glyph wins whenever there is one, because that is what the user is
 * looking at; otherwise the Font tab's selection is used. A locked-spacing text
 * tab has no target at all, so the command is disabled rather than silently
 * doing nothing.
 */
function metricsTarget(): MetricsTarget | null {
  const tab = selectActiveTab(useUiStore.getState())
  if (!tab) return null
  if (spacingLockedForDocument(tab.doc)) return null
  const active = useEditorStore.getState().activeGlyph
  if (active && active.doc === tab.doc && tab.kind !== 'font')
    return {
      doc: active.doc,
      glyphs: [active.glyph.name],
      master: active.master,
      reload: active.reload,
    }
  if (tab.kind !== 'font') return null
  const view = useDocumentStore.getState().byDoc[tab.doc]
  const selection = view?.selection ?? new Set<GlyphId>()
  if (selection.size === 0 || selection.size > MAX_SELECTION) return null
  const glyphs = glyphNamesOf(tab.doc, selection)
  return glyphs.length === 0
    ? null
    : { doc: tab.doc, glyphs, master: view?.master ?? null }
}

/** Applies one command with the same barriers a panel field uses. */
function apply(doc: DocId, command: Command, label: string): Promise<void> {
  const settling = activeEditor()?.settle()
  const earlier = pendingDocumentEdits(doc)
  const work = Promise.all([settling, earlier]).then(async () => {
    await call('doc.apply', { doc, command, label })
  })
  holdDocumentEdits(doc, work)
  return work
}

function updateMetrics(allMasters: boolean): void {
  const target = metricsTarget()
  if (!target) return
  // `UpdateMetrics { master: null }` means *every* master layer. The
  // current-master command must therefore refuse when no master is selected
  // yet, rather than quietly resynchronising the whole family.
  if (!allMasters && target.master === null) return
  const master = allMasters ? null : target.master
  const commands: Command[] = target.glyphs.map((glyph) => ({
    UpdateMetrics: { glyph, master },
  }))
  const command = commands.length === 1 ? commands[0]! : { Batch: commands }
  void apply(
    target.doc,
    command,
    i18n.t(allMasters ? 'panel.metrics.updateAll' : 'panel.metrics.update'),
  )
    .then(() => target.reload?.())
    .catch(reportError)
}

/** Registers the two metrics synchronisation commands. Safe to call twice. */
export function registerMetricsCommands(): void {
  const commands: ShellCommand[] = [
    {
      id: 'glyph.updateMetrics',
      title: 'cmd.glyph.updateMetrics',
      menu: 'glyph',
      group: 'metrics',
      shortcut: 'Mod+Alt+M',
      when: () => metricsTarget()?.master != null,
      disabledReason: 'cmd.reason.noMetricsMaster',
      run: () => updateMetrics(false),
    },
    {
      id: 'glyph.updateMetricsAll',
      title: 'cmd.glyph.updateMetricsAll',
      menu: 'glyph',
      group: 'metrics',
      shortcut: 'Mod+Alt+Shift+M',
      when: () => metricsTarget() !== null,
      disabledReason: 'cmd.reason.noMetricsTarget',
      run: () => updateMetrics(true),
    },
  ]
  registerCommands(commands)
}
