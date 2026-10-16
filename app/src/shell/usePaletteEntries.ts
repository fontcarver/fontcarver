import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { call, type SavedSmartFilter } from '../api'
import { useAutomationStore } from '../automation/automationStore'
import {
  emptyFilter,
  selectDocView,
  useDocumentStore,
} from '../stores/documentStore'
import { selectActiveDoc, useUiStore } from '../stores/uiStore'

import { runCommand } from './commands'
import {
  createPaletteFilters,
  openPaletteFilter,
  searchPaletteEntries,
  type PaletteEntry,
  type PaletteScope,
} from './paletteSearch'
import { openGlyphTab } from './tabActions'
import { useCommandList } from './useCommandRegistry'
import { usePaletteGlyphSearch } from './usePaletteGlyphSearch'

/** Collects live palette sources while keeping each remote query bounded. */
export function usePaletteEntries(query: string, scope: PaletteScope) {
  const { t } = useTranslation()
  const registered = useCommandList()
  const doc = useUiStore(selectActiveDoc)
  const view = useDocumentStore(selectDocView(doc))
  const scripts = useAutomationStore((state) => state.scripts)
  const scriptsLoading = useAutomationStore((state) => state.loading)
  const scriptsLoaded = useAutomationStore((state) => state.loaded)
  const scriptsError = useAutomationStore((state) => state.error)
  const scriptRunning = useAutomationStore((state) => state.running)
  const loadScripts = useAutomationStore((state) => state.loadScripts)
  const [saved, setSaved] = useState<{
    doc: number
    filters: SavedSmartFilter[]
    error: string | null
  } | null>(null)
  const glyphs = usePaletteGlyphSearch(
    doc,
    view.master,
    view.generation,
    query,
    scope === 'glyphs' || (scope === 'all' && query.trim().length > 0),
  )

  useEffect(() => {
    if (scope === 'all' || scope === 'scripts') void loadScripts()
  }, [loadScripts, scope])
  useEffect(() => {
    if (doc === null) return
    if (useDocumentStore.getState().byDoc[doc]?.sidebar == null)
      void useDocumentStore.getState().loadSidebar(doc)
    let current = true
    void call('glyph.smart_filters.get', { doc }).then(
      (result) => {
        if (current) setSaved({ doc, filters: result.filters, error: null })
      },
      (error: unknown) => {
        if (current)
          setSaved({
            doc,
            filters: [],
            error: error instanceof Error ? error.message : String(error),
          })
      },
    )
    return () => {
      current = false
    }
  }, [doc])

  const local: PaletteEntry[] = registered.map((command) => ({
    id: command.id,
    kind: 'commands',
    label: t(command.title),
    // A palette-only command has no menu to name, so it shows its own
    // detail instead — which is how a tool row carries its key and its hint.
    detail: command.menu ? t(`menu.${command.menu}`) : command.detail?.(),
    shortcut: command.shortcut,
    enabled: command.when?.() ?? true,
    disabledReason: command.disabledReason
      ? t(command.disabledReason)
      : undefined,
    run: () => runCommand(command.id),
  }))
  if (doc !== null) {
    local.push(...createPaletteFilters(doc, view.sidebar, t))
    if (saved?.doc === doc)
      local.push(
        ...saved.filters.map((filter): PaletteEntry => ({
          id: `smart-filter:${doc}:${filter.name}`,
          kind: 'filters',
          label: filter.name,
          detail: t('paletteSearch.savedFilter'),
          enabled: true,
          run: () =>
            openPaletteFilter(doc, {
              ...emptyFilter(),
              smart_expression: filter.expression,
            }),
        })),
      )
  }
  local.push(
    ...scripts.map((script): PaletteEntry => ({
      // A name can occur in more than one script root, so the entry identity
      // and its label both carry the root the file was found in.
      id: `script:${script.folder}:${script.name}`,
      kind: 'scripts',
      label: script.name,
      detail: t('paletteSearch.openScript'),
      enabled: !scriptRunning,
      disabledReason: scriptRunning
        ? t('paletteSearch.scriptRunning')
        : undefined,
      run: () =>
        useAutomationStore.getState().openScript(script.name, script.folder),
    })),
  )
  const entries = searchPaletteEntries(query, scope, local)
  if (doc !== null && (scope === 'all' || scope === 'glyphs')) {
    entries.push(
      ...glyphs.items.map((glyph): PaletteEntry => ({
        id: `glyph:${doc}:${glyph.gid}`,
        kind: 'glyphs',
        label: glyph.name,
        detail: glyph.unicodes
          .map(
            (value) => `U+${value.toString(16).toUpperCase().padStart(4, '0')}`,
          )
          .join(' '),
        enabled: true,
        run: () => openGlyphTab(doc, glyph.gid, glyph.name),
      })),
    )
  }
  const notices: string[] = []
  if ((scope === 'filters' || scope === 'glyphs') && doc === null)
    notices.push(t('cmd.reason.noDocument'))
  if (
    (scope === 'scripts' || scope === 'all') &&
    !scriptsLoaded &&
    scriptsError
  )
    notices.push(t('paletteSearch.scriptsError', { message: scriptsError }))
  if (
    (scope === 'filters' || scope === 'all') &&
    saved?.doc === doc &&
    saved?.error
  )
    notices.push(t('paletteSearch.filtersError', { message: saved.error }))
  if (glyphs.error)
    notices.push(t('paletteSearch.glyphsError', { message: glyphs.error }))
  if (glyphs.total > glyphs.items.length)
    notices.push(
      t('paletteSearch.moreGlyphs', {
        shown: glyphs.items.length,
        total: glyphs.total,
      }),
    )
  return {
    entries,
    notices,
    loading:
      glyphs.loading ||
      (doc !== null &&
        (scope === 'all' || scope === 'filters') &&
        saved?.doc !== doc) ||
      ((scope === 'all' || scope === 'scripts') && scriptsLoading),
  }
}
