/**
 * Resolves the design tokens the cell renderer paints with.
 *
 * Canvas cannot read CSS variables, so the token values are read once per
 * theme from the document element and handed to `paintCell` as plain colours.
 */

import { useEffect, useState } from 'react'

import { useUiStore } from '../../stores/uiStore'

import type { CellTheme } from './cellRenderer'

/** Colours used when the tokens cannot be read (tests, first paint). */
const FALLBACK: CellTheme = {
  background: '#ffffff',
  backgroundSelected: '#dde7f2',
  glyph: '#17171a',
  glyphMuted: '#6b6b74',
  label: '#5a5a63',
  ring: '#3a6ea5',
  focusRing: '#3a6ea5',
  baseline: '#e2e2e5',
  badge: '#3a6ea5',
  border: '#e2e2e5',
}

function readTheme(): CellTheme {
  const root = globalThis.document?.documentElement
  if (!root || typeof getComputedStyle !== 'function') return FALLBACK
  const style = getComputedStyle(root)
  const token = (name: string, fallback: string): string =>
    style.getPropertyValue(name).trim() || fallback
  return {
    background: token('--color-canvas', FALLBACK.background),
    backgroundSelected: token(
      '--color-accent-soft',
      FALLBACK.backgroundSelected,
    ),
    glyph: token('--color-text', FALLBACK.glyph),
    glyphMuted: token('--color-text-faint', FALLBACK.glyphMuted),
    label: token('--color-text-muted', FALLBACK.label),
    ring: token('--color-accent', FALLBACK.ring),
    focusRing: token('--color-focus-ring', FALLBACK.focusRing),
    baseline: token('--color-border', FALLBACK.baseline),
    badge: token('--color-accent', FALLBACK.badge),
    border: token('--color-border-subtle', FALLBACK.border),
  }
}

/** The UI font stack the cell labels are drawn in. */
export function uiFontFamily(): string {
  const root = globalThis.document?.documentElement
  if (!root || typeof getComputedStyle !== 'function') return 'system-ui'
  return (
    getComputedStyle(root).getPropertyValue('--font-ui').trim() || 'system-ui'
  )
}

/**
 * The current cell colours. Re-read after the shell has written `data-theme`,
 * which happens in a parent effect, so the read is deferred by one frame.
 */
export function useCellTheme(): CellTheme {
  const preference = useUiStore((state) => state.theme)
  const [theme, setTheme] = useState<CellTheme>(FALLBACK)

  useEffect(() => {
    let cancelled = false
    const refresh = () => {
      if (!cancelled) setTheme(readTheme())
    }
    const frame =
      typeof requestAnimationFrame === 'function'
        ? requestAnimationFrame(refresh)
        : undefined
    if (frame === undefined) refresh()

    const query = globalThis.matchMedia?.('(prefers-color-scheme: dark)')
    query?.addEventListener('change', refresh)
    return () => {
      cancelled = true
      if (frame !== undefined) cancelAnimationFrame(frame)
      query?.removeEventListener('change', refresh)
    }
  }, [preference])

  return theme
}
