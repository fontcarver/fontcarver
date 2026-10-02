import { useKeymapStore } from '../stores/keymapStore'
import { canonicalShortcut, type KeymapOverrides } from './keymapDocument'
import { isMacPlatform } from './shortcutSyntax'

export type { KeymapOverrides } from './keymapDocument'

/** An action's effective binding, including explicit unbinding. */
export function shortcutFor(
  id: string,
  fallback?: string,
  overrides: KeymapOverrides = useKeymapStore.getState().overrides,
): string | null {
  return Object.hasOwn(overrides, id)
    ? (overrides[id] ?? null)
    : (fallback ?? null)
}

/** Action metadata for a command or tap/hold tool. */
export interface KeymapTarget {
  id: string
  title: string
  shortcut?: string
  scope: string
}

/** A group of actions whose bindings overlap in at least one context. */
export interface KeymapConflict {
  shortcut: string
  ids: string[]
}

/** Check potential contexts, rather than hiding conflicts behind current guards. */
export function keymapConflicts(
  targets: readonly KeymapTarget[],
  overrides: KeymapOverrides,
  mac: boolean = isMacPlatform(),
): KeymapConflict[] {
  const byKey = new Map<string, KeymapTarget[]>()
  for (const target of targets) {
    const binding = shortcutFor(target.id, target.shortcut, overrides)
    const canonical = binding && canonicalShortcut(binding)
    // Compare physical overlaps without rewriting the saved binding: Control
    // and Mod are different on macOS, but are the same key on Windows/Linux.
    const key = mac ? canonical : canonical?.replace(/^Control\+/, 'Mod+')
    if (!key) continue
    byKey.set(key, [...(byKey.get(key) ?? []), target])
  }
  const conflicts: KeymapConflict[] = []
  for (const [shortcut, rows] of byKey) {
    const overlapping = rows.filter((row) =>
      rows.some(
        (other) =>
          other.id !== row.id &&
          (row.scope === 'global' ||
            other.scope === 'global' ||
            row.scope === other.scope),
      ),
    )
    if (overlapping.length > 1)
      conflicts.push({ shortcut, ids: overlapping.map((row) => row.id) })
  }
  return conflicts
}

/**
 * The conflicts of each action, keyed by action id.
 *
 * The editor marks a row from this rather than from the flat list, so a
 * conflicting binding is visible on the row the reader is editing and not only
 * in a summary at the top of the dialog.
 */
export function conflictsByTarget(
  conflicts: readonly KeymapConflict[],
): Map<string, KeymapConflict[]> {
  const byId = new Map<string, KeymapConflict[]>()
  for (const conflict of conflicts)
    for (const id of conflict.ids)
      byId.set(id, [...(byId.get(id) ?? []), conflict])
  return byId
}

/**
 * The conflicts that still block saving: the ones whose chord the user has not
 * deliberately accepted.
 */
export function blockingConflicts(
  conflicts: readonly KeymapConflict[],
  allowed: readonly string[],
): KeymapConflict[] {
  const accepted = new Set(allowed)
  return conflicts.filter((conflict) => !accepted.has(conflict.shortcut))
}
