/**
 * One help entry per registered command.
 *
 * Row 107 of the P5 exit checklist asks that every command in the registry have
 * a help entry. The mapping is explicit and total: there is no fallback, and
 * `commandHelp.test.ts` fails when a newly registered command has no entry,
 * when an entry names a topic that does not exist, or when an entry is stale.
 *
 * An entry carries the article the command belongs to and the i18n key of the
 * one-line description the Quick Help bubble, the menu tooltip and the palette
 * read. Families whose members differ only in their argument (the twelve colour
 * labels, the nine numbered tab shortcuts, the theme and language choices and
 * the two text-kerning gestures) share one entry through {@link familyOf}.
 */

import type { HelpTopic } from './contextHelp'

/** The help article and the one-line description of one command. */
export interface CommandHelp {
  /** Article `openContextHelp` opens for this command. */
  readonly topic: HelpTopic
  /** i18n key of the one-line description shown as quick help. */
  readonly summary: string
}

/**
 * Commands whose members share one entry, as `id prefix -> entry id`. Checked
 * before the exact table, so `glyph.colorLabel.none` still gets its own entry.
 */
const FAMILIES: readonly [string, string][] = [
  ['glyph.colorLabel.', 'glyph.colorLabel'],
  ['window.tab.', 'window.tab'],
  ['view.theme.', 'view.theme'],
  ['view.language.', 'view.language'],
  ['text.kerning.', 'text.kerning'],
]

/** Every command id, in menu order. Keep in step with the `quickHelp` keys. */
const ENTRIES: Readonly<Record<string, CommandHelp>> = {
  'file.new': { topic: 'newFont', summary: 'quickHelp.file.new' },
  'file.open': { topic: 'files', summary: 'quickHelp.file.open' },
  'file.openProject': { topic: 'files', summary: 'quickHelp.file.openProject' },
  'file.openRecent': { topic: 'files', summary: 'quickHelp.file.openRecent' },
  'file.save': { topic: 'files', summary: 'quickHelp.file.save' },
  'file.saveAs': { topic: 'files', summary: 'quickHelp.file.saveAs' },
  'file.close': { topic: 'files', summary: 'quickHelp.file.close' },
  'file.fontInfo': { topic: 'fontinfo', summary: 'quickHelp.file.fontInfo' },
  'file.export': { topic: 'export', summary: 'quickHelp.file.export' },
  'edit.undo': { topic: 'history', summary: 'quickHelp.edit.undo' },
  'edit.redo': { topic: 'history', summary: 'quickHelp.edit.redo' },
  'edit.history': { topic: 'history', summary: 'quickHelp.edit.history' },
  'edit.preferences': {
    topic: 'preferences',
    summary: 'quickHelp.edit.preferences',
  },
  'edit.selectAll': { topic: 'font', summary: 'quickHelp.edit.selectAll' },
  'edit.selectNone': { topic: 'font', summary: 'quickHelp.edit.selectNone' },
  'edit.copyGlyphNames': {
    topic: 'font',
    summary: 'quickHelp.edit.copyGlyphNames',
  },
  'edit.selectAllPoints': {
    topic: 'outline',
    summary: 'quickHelp.edit.selectAllPoints',
  },
  'edit.deselectPoints': {
    topic: 'outline',
    summary: 'quickHelp.edit.deselectPoints',
  },
  'edit.deletePoints': {
    topic: 'outline',
    summary: 'quickHelp.edit.deletePoints',
  },
  'edit.deletePointsBreak': {
    topic: 'outline',
    summary: 'quickHelp.edit.deletePointsBreak',
  },
  'edit.pasteSpecial': {
    topic: 'crossDocument',
    summary: 'quickHelp.edit.pasteSpecial',
  },
  'edit.relinkComponents': {
    topic: 'crossDocument',
    summary: 'quickHelp.edit.relinkComponents',
  },
  'font.findProblems': {
    topic: 'problems',
    summary: 'quickHelp.font.findProblems',
  },
  'font.compare': { topic: 'crossDocument', summary: 'quickHelp.font.compare' },
  'font.styleEffects': {
    topic: 'outline',
    summary: 'quickHelp.font.styleEffects',
  },
  'glyph.open': { topic: 'glyph', summary: 'quickHelp.glyph.open' },
  'glyph.previous': { topic: 'glyph', summary: 'quickHelp.glyph.previous' },
  'glyph.next': { topic: 'glyph', summary: 'quickHelp.glyph.next' },
  'glyph.add': { topic: 'addGlyphs', summary: 'quickHelp.glyph.add' },
  'glyph.duplicate': { topic: 'glyph', summary: 'quickHelp.glyph.duplicate' },
  'glyph.renameBatch': {
    topic: 'batchRename',
    summary: 'quickHelp.glyph.renameBatch',
  },
  'glyph.updateInfo': { topic: 'font', summary: 'quickHelp.glyph.updateInfo' },
  'glyph.updateMetrics': {
    topic: 'spacing',
    summary: 'quickHelp.glyph.updateMetrics',
  },
  'glyph.updateMetricsAll': {
    topic: 'spacing',
    summary: 'quickHelp.glyph.updateMetricsAll',
  },
  'glyph.decomposeAll': {
    topic: 'outline',
    summary: 'quickHelp.glyph.decomposeAll',
  },
  'glyph.alignComponents': {
    topic: 'outline',
    summary: 'quickHelp.glyph.alignComponents',
  },
  'glyph.colorLabel': { topic: 'font', summary: 'quickHelp.glyph.colorLabel' },
  'glyph.colorLabel.none': {
    topic: 'font',
    summary: 'quickHelp.glyph.colorLabelNone',
  },
  'path.removeOverlap': {
    topic: 'outline',
    summary: 'quickHelp.path.removeOverlap',
  },
  'path.correctDirection': {
    topic: 'outline',
    summary: 'quickHelp.path.correctDirection',
  },
  'path.correctDirectionAllMasters': {
    topic: 'outline',
    summary: 'quickHelp.path.correctDirectionAllMasters',
  },
  'path.addExtrema': { topic: 'outline', summary: 'quickHelp.path.addExtrema' },
  'path.forceExtrema': {
    topic: 'outline',
    summary: 'quickHelp.path.forceExtrema',
  },
  'path.addInflections': {
    topic: 'outline',
    summary: 'quickHelp.path.addInflections',
  },
  'path.simplify': { topic: 'outline', summary: 'quickHelp.path.simplify' },
  'path.roundToGrid': {
    topic: 'outline',
    summary: 'quickHelp.path.roundToGrid',
  },
  'path.tidy': { topic: 'outline', summary: 'quickHelp.path.tidy' },
  'path.tidyAllMasters': {
    topic: 'outline',
    summary: 'quickHelp.path.tidyAllMasters',
  },
  'path.expandStroke': {
    topic: 'outline',
    summary: 'quickHelp.path.expandStroke',
  },
  'path.convertCubic': {
    topic: 'outline',
    summary: 'quickHelp.path.convertCubic',
  },
  'path.convertQuadratic': {
    topic: 'outline',
    summary: 'quickHelp.path.convertQuadratic',
  },
  'path.reverse': { topic: 'outline', summary: 'quickHelp.path.reverse' },
  'path.setStartPoint': {
    topic: 'outline',
    summary: 'quickHelp.path.setStartPoint',
  },
  'path.join': { topic: 'outline', summary: 'quickHelp.path.join' },
  'path.break': { topic: 'outline', summary: 'quickHelp.path.break' },
  'path.openCorner': { topic: 'outline', summary: 'quickHelp.path.openCorner' },
  'path.reconnectNodes': {
    topic: 'outline',
    summary: 'quickHelp.path.reconnectNodes',
  },
  'path.toggleSmooth': {
    topic: 'outline',
    summary: 'quickHelp.path.toggleSmooth',
  },
  'path.toggleSmoothKeepOutline': {
    topic: 'outline',
    summary: 'quickHelp.path.toggleSmoothKeepOutline',
  },
  'path.superSmooth': {
    topic: 'outline',
    summary: 'quickHelp.path.superSmooth',
  },
  'path.harmonize': { topic: 'outline', summary: 'quickHelp.path.harmonize' },
  'layer.editBackground': {
    topic: 'outline',
    summary: 'quickHelp.layer.editBackground',
  },
  'layer.selectionToBackground': {
    topic: 'outline',
    summary: 'quickHelp.layer.selectionToBackground',
  },
  'layer.addBackground': {
    topic: 'outline',
    summary: 'quickHelp.layer.addBackground',
  },
  'layer.copyToBackground': {
    topic: 'outline',
    summary: 'quickHelp.layer.copyToBackground',
  },
  'layer.swapWithBackground': {
    topic: 'outline',
    summary: 'quickHelp.layer.swapWithBackground',
  },
  'layer.clearBackground': {
    topic: 'outline',
    summary: 'quickHelp.layer.clearBackground',
  },
  'layer.addBackup': { topic: 'outline', summary: 'quickHelp.layer.addBackup' },
  'view.toggleSidebar': {
    topic: 'workspace',
    summary: 'quickHelp.view.toggleSidebar',
  },
  'view.toggleInspector': {
    topic: 'workspace',
    summary: 'quickHelp.view.toggleInspector',
  },
  'view.zoomIn': { topic: 'font', summary: 'quickHelp.view.zoomIn' },
  'view.zoomOut': { topic: 'font', summary: 'quickHelp.view.zoomOut' },
  'view.zoomFit': { topic: 'glyph', summary: 'quickHelp.view.zoomFit' },
  'view.features': { topic: 'features', summary: 'quickHelp.view.features' },
  'view.textPreview': { topic: 'glyph', summary: 'quickHelp.view.textPreview' },
  'view.previewCollapse': {
    topic: 'glyph',
    summary: 'quickHelp.view.previewCollapse',
  },
  'view.toggleInfoBox': {
    topic: 'glyph',
    summary: 'quickHelp.view.toggleInfoBox',
  },
  'view.toggleSnapping': {
    topic: 'glyph',
    summary: 'quickHelp.view.toggleSnapping',
  },
  'view.toggleFill': { topic: 'glyph', summary: 'quickHelp.view.toggleFill' },
  'view.glyph.filled': {
    topic: 'glyph',
    summary: 'quickHelp.view.glyph.filled',
  },
  'view.glyph.points': {
    topic: 'glyph',
    summary: 'quickHelp.view.glyph.points',
  },
  'view.glyph.metrics': {
    topic: 'glyph',
    summary: 'quickHelp.view.glyph.metrics',
  },
  'view.glyph.anchors': {
    topic: 'glyph',
    summary: 'quickHelp.view.glyph.anchors',
  },
  'view.glyph.guides': {
    topic: 'glyph',
    summary: 'quickHelp.view.glyph.guides',
  },
  'view.glyph.rulers': {
    topic: 'glyph',
    summary: 'quickHelp.view.glyph.rulers',
  },
  'view.glyph.background': {
    topic: 'glyph',
    summary: 'quickHelp.view.glyph.background',
  },
  'view.glyph.otherMasters': {
    topic: 'glyph',
    summary: 'quickHelp.view.glyph.otherMasters',
  },
  'view.glyph.verticalMode': {
    topic: 'glyph',
    summary: 'quickHelp.view.glyph.verticalMode',
  },
  'view.theme': { topic: 'preferences', summary: 'quickHelp.view.theme' },
  'view.language': { topic: 'preferences', summary: 'quickHelp.view.language' },
  'bitmap.open': { topic: 'bitmap', summary: 'quickHelp.bitmap.open' },
  'bitmap.tool.pencil': {
    topic: 'bitmap',
    summary: 'quickHelp.bitmap.tool.pencil',
  },
  'bitmap.tool.erase': {
    topic: 'bitmap',
    summary: 'quickHelp.bitmap.tool.erase',
  },
  'bitmap.tool.line': {
    topic: 'bitmap',
    summary: 'quickHelp.bitmap.tool.line',
  },
  'bitmap.tool.rectangle': {
    topic: 'bitmap',
    summary: 'quickHelp.bitmap.tool.rectangle',
  },
  'bitmap.tool.ellipse': {
    topic: 'bitmap',
    summary: 'quickHelp.bitmap.tool.ellipse',
  },
  'bitmap.tool.fill': {
    topic: 'bitmap',
    summary: 'quickHelp.bitmap.tool.fill',
  },
  'bitmap.tool.select': {
    topic: 'bitmap',
    summary: 'quickHelp.bitmap.tool.select',
  },
  'bitmap.tool.eyedropper': {
    topic: 'bitmap',
    summary: 'quickHelp.bitmap.tool.eyedropper',
  },
  'bitmap.tool.hand': {
    topic: 'bitmap',
    summary: 'quickHelp.bitmap.tool.hand',
  },
  'bitmap.overlay.grid': {
    topic: 'bitmap',
    summary: 'quickHelp.bitmap.overlay.grid',
  },
  'bitmap.overlay.outline': {
    topic: 'bitmap',
    summary: 'quickHelp.bitmap.overlay.outline',
  },
  'bitmap.overlay.advanceColumn': {
    topic: 'bitmap',
    summary: 'quickHelp.bitmap.overlay.advanceColumn',
  },
  'bitmap.overlay.fontUnits': {
    topic: 'bitmap',
    summary: 'quickHelp.bitmap.overlay.fontUnits',
  },
  'automation.console': {
    topic: 'automation',
    summary: 'quickHelp.automation.console',
  },
  'palette.open': { topic: 'shortcuts', summary: 'quickHelp.palette.open' },
  'palette.quickOpen': {
    topic: 'shortcuts',
    summary: 'quickHelp.palette.quickOpen',
  },
  'window.nextTab': { topic: 'workspace', summary: 'quickHelp.window.nextTab' },
  'window.previousTab': {
    topic: 'workspace',
    summary: 'quickHelp.window.previousTab',
  },
  'window.cycleNext': {
    topic: 'workspace',
    summary: 'quickHelp.window.cycleNext',
  },
  'window.cyclePrevious': {
    topic: 'workspace',
    summary: 'quickHelp.window.cyclePrevious',
  },
  'window.closeTab': {
    topic: 'workspace',
    summary: 'quickHelp.window.closeTab',
  },
  'window.tab': { topic: 'workspace', summary: 'quickHelp.window.tab' },
  'window.detachTab': {
    topic: 'workspace',
    summary: 'quickHelp.window.detachTab',
  },
  'window.detachSidebar': {
    topic: 'workspace',
    summary: 'quickHelp.window.detachSidebar',
  },
  'window.detachInspector': {
    topic: 'workspace',
    summary: 'quickHelp.window.detachInspector',
  },
  'window.detachPreview': {
    topic: 'workspace',
    summary: 'quickHelp.window.detachPreview',
  },
  'window.document': {
    topic: 'workspace',
    summary: 'quickHelp.window.document',
  },
  'workspace.manage': {
    topic: 'workspace',
    summary: 'quickHelp.workspace.manage',
  },
  'help.context': { topic: 'shortcuts', summary: 'quickHelp.help.context' },
  'help.shortcuts': { topic: 'shortcuts', summary: 'quickHelp.help.shortcuts' },
  'help.about': { topic: 'welcome', summary: 'quickHelp.help.about' },
  'text.kerning': { topic: 'spacing', summary: 'quickHelp.text.kerning' },
  'tool.select': { topic: 'glyph', summary: 'quickHelp.tool.select' },
  'tool.lasso': { topic: 'glyph', summary: 'quickHelp.tool.lasso' },
  'tool.eraser': { topic: 'glyph', summary: 'quickHelp.tool.eraser' },
  'tool.magnet': { topic: 'glyph', summary: 'quickHelp.tool.magnet' },
  'tool.annotation': { topic: 'glyph', summary: 'quickHelp.tool.annotation' },
  'tool.pen': { topic: 'glyph', summary: 'quickHelp.tool.pen' },
  'tool.pencil': { topic: 'glyph', summary: 'quickHelp.tool.pencil' },
  'tool.knife': { topic: 'glyph', summary: 'quickHelp.tool.knife' },
  'tool.rectangle': { topic: 'glyph', summary: 'quickHelp.tool.rectangle' },
  'tool.ellipse': { topic: 'glyph', summary: 'quickHelp.tool.ellipse' },
  'tool.measure': { topic: 'glyph', summary: 'quickHelp.tool.measure' },
  'tool.rotate': { topic: 'glyph', summary: 'quickHelp.tool.rotate' },
  'tool.scale': { topic: 'glyph', summary: 'quickHelp.tool.scale' },
  'tool.zoom': { topic: 'glyph', summary: 'quickHelp.tool.zoom' },
  'tool.hand': { topic: 'glyph', summary: 'quickHelp.tool.hand' },
  'tool.text': { topic: 'glyph', summary: 'quickHelp.tool.text' },
  'tool.pixel': { topic: 'bitmap', summary: 'quickHelp.tool.pixel' },
}

/** The family entry id for a command whose members share one description. */
function familyOf(id: string): string | undefined {
  return FAMILIES.find(([prefix]) => id.startsWith(prefix))?.[1]
}

/** The help entry of one command, or `undefined` when the id is unknown. */
export function commandHelp(id: string): CommandHelp | undefined {
  const exact = Object.hasOwn(ENTRIES, id) ? ENTRIES[id] : undefined
  if (exact) return exact
  const family = familyOf(id)
  return family !== undefined && Object.hasOwn(ENTRIES, family)
    ? ENTRIES[family]
    : undefined
}

/** Every command id that has an entry. Used by the coverage test. */
export function commandHelpIds(): string[] {
  return Object.keys(ENTRIES)
}
