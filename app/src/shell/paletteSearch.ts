import type { TFunction } from 'i18next'

import type { DocId, GlyphFilter, SidebarCounts } from '../api'
import { emptyFilter, useDocumentStore } from '../stores/documentStore'
import { useSessionStore } from '../stores/sessionStore'
import { useUiStore } from '../stores/uiStore'
import { colorLabelKey } from '../views/font/colorLabels'

import { fuzzyRank } from './fuzzy'
import { prepareTabTransition } from './tabActions'

/** Content categories supported by the palette. */
export type PaletteScope = 'all' | 'commands' | 'filters' | 'scripts' | 'glyphs'

/** A resolved palette action with its current display and enablement state. */
export interface PaletteEntry {
  id: string
  kind: Exclude<PaletteScope, 'all'>
  label: string
  detail?: string
  shortcut?: string
  enabled: boolean
  disabledReason?: string
  run: () => unknown | Promise<unknown>
}

/** Display order of the palette's sections. */
export const PALETTE_SECTIONS = [
  'commands',
  'filters',
  'scripts',
  'glyphs',
] as const satisfies readonly Exclude<PaletteScope, 'all'>[]

/** Limits the selected category and fuzzy-ranks its translated labels. */
export function searchPaletteEntries(
  query: string,
  scope: PaletteScope,
  entries: readonly PaletteEntry[],
): PaletteEntry[] {
  return fuzzyRank(
    query,
    entries.filter((entry) => scope === 'all' || entry.kind === scope),
    (entry) => entry.label,
  )
}

/**
 * Groups ranked results into the palette's sections.
 *
 * Ranking happens across every category first, so a good match is still a good
 * match; the sections only decide where an equally good match is shown. Within
 * a section the ranked order is preserved, and empty sections are dropped.
 */
export function paletteSections(
  entries: readonly PaletteEntry[],
): { kind: Exclude<PaletteScope, 'all'>; entries: PaletteEntry[] }[] {
  return PALETTE_SECTIONS.map((kind) => ({
    kind,
    entries: entries.filter((entry) => entry.kind === kind),
  })).filter((section) => section.entries.length > 0)
}

/** Replaces the current overview filter after the active editor settles. */
export async function openPaletteFilter(
  doc: DocId,
  filter: GlyphFilter,
): Promise<void> {
  if (!useSessionStore.getState().docs.some((summary) => summary.doc === doc))
    return
  const transition = prepareTabTransition()
  if (!(await transition.ready) || !transition.isCurrent()) return
  if (!useSessionStore.getState().docs.some((summary) => summary.doc === doc))
    return
  useDocumentStore.getState().setFilter(doc, filter)
  useUiStore.getState().openTab({ kind: 'font', doc })
}

/** Builds searchable overview actions from the current font taxonomy. */
export function createPaletteFilters(
  doc: DocId,
  counts: SidebarCounts | null,
  t: TFunction,
): PaletteEntry[] {
  const filterEntry = (
    id: string,
    label: string,
    patch: Partial<GlyphFilter>,
    detail?: string,
  ): PaletteEntry => ({
    id: `filter:${doc}:${id}`,
    kind: 'filters',
    label,
    detail,
    enabled: true,
    run: () => openPaletteFilter(doc, { ...emptyFilter(), ...patch }),
  })
  return [
    filterEntry('all', t('sidebar.all'), {}),
    filterEntry('unencoded', t('sidebar.unencoded'), { unencoded_only: true }),
    filterEntry('changed', t('paletteSearch.changed'), { changed_only: true }),
    filterEntry('components', t('paletteSearch.components'), {
      has_components: true,
    }),
    ...(counts?.categories ?? []).map(([name]) =>
      filterEntry(
        `category:${name}`,
        name,
        { categories: [name] },
        t('sidebar.categories'),
      ),
    ),
    ...(counts?.scripts ?? []).map(([name]) =>
      filterEntry(
        `script:${name}`,
        name,
        { scripts: [name] },
        t('sidebar.scripts'),
      ),
    ),
    ...(counts?.blocks ?? []).map(([name]) =>
      filterEntry(
        `block:${name}`,
        name,
        { blocks: [name] },
        t('sidebar.blocks'),
      ),
    ),
    ...(counts?.color_labels ?? []).map(([index]) =>
      filterEntry(
        `color:${index}`,
        t(`fontView.color.${colorLabelKey(index)}`),
        { color_labels: [index] },
        t('sidebar.colors'),
      ),
    ),
  ]
}
