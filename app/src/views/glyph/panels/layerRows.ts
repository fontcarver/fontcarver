/**
 * The Layers panel's row model (design spec §7.3).
 *
 * Glyph layers arrive in stored order, which is whatever the source format
 * happened to use. The panel shows them grouped by role — masters first, then
 * the intermediate layers, then backgrounds, then backups newest first — so
 * the list reads the same way in every font. The grouping is pure so it can be
 * asserted without rendering the panel.
 */

import type { Layer, LayerKind, MasterSummary } from '../../../api'

/** The role groups, in the order the panel lists them. */
export type LayerGroup = 'master' | 'intermediate' | 'background' | 'backup'

/** One row of the Layers panel. */
export interface LayerRow {
  layer: Layer
  group: LayerGroup
  /** i18n key naming the role. */
  kindKey: string
  /** Master the layer belongs to, or `null` for a free-standing layer. */
  master: string | null
  /** Backup timestamp in seconds, for the rows that carry one. */
  time: number | null
  /** Master layers are structural: `RemoveLayer` refuses to delete them. */
  removable: boolean
}

/** The i18n key naming a layer's role. */
export function layerKindKey(kind: LayerKind): string {
  if ('Master' in kind) return 'master'
  if ('Brace' in kind) return 'brace'
  if ('Smart' in kind) return 'smart'
  if ('Bracket' in kind) return 'bracket'
  if ('Background' in kind) return 'background'
  if ('Backup' in kind) return 'backup'
  return 'custom'
}

/** The master a layer belongs to, or `null` for a custom layer. */
export function layerMaster(kind: LayerKind): string | null {
  if ('Master' in kind) return kind.Master
  if ('Brace' in kind) return kind.Brace.master
  if ('Smart' in kind) return kind.Smart.master
  if ('Bracket' in kind) return kind.Bracket.master
  if ('Background' in kind) return kind.Background
  if ('Backup' in kind) return kind.Backup.of
  return null
}

function groupOf(kind: LayerKind): LayerGroup {
  if ('Master' in kind) return 'master'
  if ('Background' in kind) return 'background'
  if ('Backup' in kind) return 'backup'
  return 'intermediate'
}

const ORDER: Record<LayerGroup, number> = {
  master: 0,
  intermediate: 1,
  background: 2,
  backup: 3,
}

/** The glyph's layers as rows, grouped and ordered for the panel. */
export function layerRows(layers: readonly Layer[]): LayerRow[] {
  const rows = layers.map((layer): LayerRow => {
    const group = groupOf(layer.kind)
    return {
      layer,
      group,
      kindKey: layerKindKey(layer.kind),
      master: layerMaster(layer.kind),
      time: 'Backup' in layer.kind ? layer.kind.Backup.time : null,
      removable: group !== 'master',
    }
  })
  return rows.sort(
    (a, b) =>
      ORDER[a.group] - ORDER[b.group] ||
      // Backups newest first; everything else keeps its stored order.
      (b.time ?? 0) - (a.time ?? 0),
  )
}

/**
 * What a row is called.
 *
 * A master layer is known by its own name (or its master's, since imported
 * fonts leave layers unnamed). A background or a backup is known by its role:
 * `CopyLayer` carries the source layer's name across, so three rows would
 * otherwise all read "Book" and only the second column would tell them apart.
 */
export function layerLabel(
  row: LayerRow,
  masters: readonly MasterSummary[],
  roleName: string,
): string {
  if (row.group === 'background' || row.group === 'backup') return roleName
  if (row.layer.name) return row.layer.name
  const master = masters.find((entry) => entry.id === row.master)
  return master ? master.name : roleName
}

/** What a row's second column shows: its master, its timestamp or its role. */
export function layerDetail(
  row: LayerRow,
  masters: readonly MasterSummary[],
  roleName: string,
  time: (seconds: number) => string,
): string {
  if (row.time !== null) return time(row.time)
  // A background says which master it belongs to, because that is the thing
  // it is a background *of*; every other row says what it is.
  if (row.group === 'background')
    return masters.find((entry) => entry.id === row.master)?.name ?? roleName
  return roleName
}
