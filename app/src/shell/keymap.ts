import { useKeymapStore } from '../stores/keymapStore'
import { canonicalShortcut, type KeymapOverrides } from './keymapDocument'

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
): KeymapConflict[] {
  const byKey = new Map<string, KeymapTarget[]>()
  for (const target of targets) {
    const binding = shortcutFor(target.id, target.shortcut, overrides)
    const key = binding && canonicalShortcut(binding)
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
