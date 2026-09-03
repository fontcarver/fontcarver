/**
 * Colours and type used by the glyph-editor drawing modules.
 *
 * Canvas cannot read CSS variables, so the design tokens are resolved once per
 * theme from the document element and handed to the draw functions as plain
 * strings. The fallback palette keeps the renderer working in tests and on the
 * very first paint, before the shell has written `data-theme`.
 */

import { useEffect, useState } from 'react'

import { useUiStore } from '../../stores/uiStore'

/** Every colour the glyph canvas paints with. */
export interface GlyphTheme {
  background: string
  fill: string
  outline: string
  metric: string
  metricStrong: string
  metricLabel: string
  sidebearing: string
  nodeCorner: string
  nodeSmooth: string
  nodeOffCurve: string
  handle: string
  start: string
  component: string
  anchor: string
  guideLocal: string
  guideGlobal: string
  rulerBackground: string
  rulerLine: string
  rulerLabel: string
  fontFamily: string
}

/** Colours used when the tokens cannot be read (tests, first paint). */
export const FALLBACK_GLYPH_THEME: GlyphTheme = {
  background: '#ffffff',
  fill: '#17171a',
  outline: '#3f3f47',
  metric: '#b9b9c0',
  metricStrong: '#8d8d96',
  metricLabel: '#6b6b74',
  sidebearing: '#9ab0c8',
  nodeCorner: '#2f5f9e',
  nodeSmooth: '#2c7a4b',
  nodeOffCurve: '#7b7b85',
  handle: '#a8a8b2',
  start: '#2f5f9e',
  component: '#9a9aa4',
  anchor: '#a3417c',
  guideLocal: '#3a6ea5',
  guideGlobal: '#a33a35',
  rulerBackground: '#ededf0',
  rulerLine: '#cfcfd3',
  rulerLabel: '#6b6b74',
  fontFamily: 'system-ui',
}

/** Reads the current token values off the document element. */
export function readGlyphTheme(): GlyphTheme {
  const root = globalThis.document?.documentElement
  if (!root || typeof getComputedStyle !== 'function')
    return FALLBACK_GLYPH_THEME
  const style = getComputedStyle(root)
  const token = (name: string, fallback: string): string =>
    style.getPropertyValue(name).trim() || fallback
  const fb = FALLBACK_GLYPH_THEME
  return {
    background: token('--color-canvas', fb.background),
    fill: token('--color-glyph-fill', fb.fill),
    outline: token('--color-glyph-outline', fb.outline),
    metric: token('--color-glyph-metric', fb.metric),
    metricStrong: token('--color-glyph-metric-strong', fb.metricStrong),
    metricLabel: token('--color-text-faint', fb.metricLabel),
    sidebearing: token('--color-glyph-sidebearing', fb.sidebearing),
    nodeCorner: token('--color-glyph-node-corner', fb.nodeCorner),
    nodeSmooth: token('--color-glyph-node-smooth', fb.nodeSmooth),
    nodeOffCurve: token('--color-glyph-node-offcurve', fb.nodeOffCurve),
    handle: token('--color-glyph-handle', fb.handle),
    start: token('--color-glyph-start', fb.start),
    component: token('--color-glyph-component', fb.component),
    anchor: token('--color-glyph-anchor', fb.anchor),
    guideLocal: token('--color-glyph-guide-local', fb.guideLocal),
    guideGlobal: token('--color-glyph-guide-global', fb.guideGlobal),
    rulerBackground: token('--color-glyph-ruler', fb.rulerBackground),
    rulerLine: token('--color-border', fb.rulerLine),
    rulerLabel: token('--color-text-faint', fb.rulerLabel),
    fontFamily: token('--font-ui', fb.fontFamily),
  }
}

/**
 * The current canvas palette. Re-read one frame after a theme change, which is
 * when the shell has written `data-theme` onto the document element.
 */
export function useGlyphTheme(): GlyphTheme {
  const preference = useUiStore((state) => state.theme)
  const [theme, setTheme] = useState<GlyphTheme>(FALLBACK_GLYPH_THEME)

  useEffect(() => {
    let cancelled = false
    const refresh = () => {
      if (!cancelled) setTheme(readGlyphTheme())
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
