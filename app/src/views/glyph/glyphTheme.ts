/**
 * Colours and type used by the glyph-editor drawing modules.
 *
 * Canvas cannot read CSS variables, so the design tokens (`--cv-*`, design
 * spec §2.1) are resolved once per theme from the document element and handed
 * to the draw functions as plain strings. The fallback palette keeps the
 * renderer working in tests and on the very first paint, before the shell has
 * written `data-theme`.
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
  nodeImplied: string
  nodeOffCurve: string
  handle: string
  start: string
  component: string
  /** The background layer's outline and its 3 px point dots. */
  backgroundLayer: string
  /** The same glyph in the other masters. */
  otherMaster: string
  anchor: string
  guideLocal: string
  guideGlobal: string
  rulerBackground: string
  rulerLine: string
  rulerLabel: string
  /** Selected nodes, handles, segments and the marquee fill. */
  selection: string
  /** The 1 px inner ring drawn inside a selected node. */
  selectionInk: string
  /** Marquee outline, snap halo and alignment lines. */
  accent: string
  /** Cursor HUD pill background and its text. */
  hudBackground: string
  hudInk: string
  /** Duplicate nodes and audit markers. */
  warning: string
  /** Alignment zones, and the diamond around a node on a vertical metric. */
  zone: string
  fontFamily: string
}

/** Colours used when the tokens cannot be read (tests, first paint). */
export const FALLBACK_GLYPH_THEME: GlyphTheme = {
  background: '#ffffff',
  fill: '#1b1f24',
  outline: '#1b1f24',
  metric: '#b8bec7',
  metricStrong: '#7a828e',
  metricLabel: '#7a828e',
  sidebearing: '#9ab0c8',
  nodeCorner: '#3b6fb6',
  nodeSmooth: '#2e8b57',
  nodeImplied: '#2f7f8f',
  nodeOffCurve: '#8e96a2',
  handle: '#8e96a2',
  start: '#3b6fb6',
  component: '#8e96a2',
  backgroundLayer: '#c9a227',
  otherMaster: '#9e7bb5',
  anchor: '#c2413b',
  guideLocal: '#3b6fb6',
  guideGlobal: '#c2413b',
  rulerBackground: '#f8f9fa',
  rulerLine: '#d9dde3',
  rulerLabel: '#7a828e',
  selection: '#2b5fa8',
  selectionInk: '#ffffff',
  accent: '#3b6fb6',
  hudBackground: 'rgb(27 31 36 / 92%)',
  hudInk: '#ffffff',
  warning: '#c2413b',
  zone: '#eadfb8',
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
    background: token('--cv-bg', fb.background),
    fill: token('--cv-ink', fb.fill),
    outline: token('--cv-ink', fb.outline),
    metric: token('--cv-metric', fb.metric),
    metricStrong: token('--cv-metric-label', fb.metricStrong),
    metricLabel: token('--cv-metric-label', fb.metricLabel),
    sidebearing: token('--cv-sidebearing', fb.sidebearing),
    nodeCorner: token('--cv-node-corner', fb.nodeCorner),
    nodeSmooth: token('--cv-node-smooth', fb.nodeSmooth),
    nodeImplied: token('--cv-node-implied', fb.nodeImplied),
    nodeOffCurve: token('--cv-handle', fb.nodeOffCurve),
    handle: token('--cv-handle', fb.handle),
    start: token('--cv-node-corner', fb.start),
    component: token('--cv-component', fb.component),
    backgroundLayer: token('--cv-background-layer', fb.backgroundLayer),
    otherMaster: token('--cv-other-master', fb.otherMaster),
    anchor: token('--cv-anchor', fb.anchor),
    guideLocal: token('--cv-guide-local', fb.guideLocal),
    guideGlobal: token('--cv-guide-global', fb.guideGlobal),
    rulerBackground: token('--cv-ruler', fb.rulerBackground),
    rulerLine: token('--color-border', fb.rulerLine),
    rulerLabel: token('--cv-metric-label', fb.rulerLabel),
    selection: token('--cv-selection', fb.selection),
    selectionInk: token('--cv-selection-ink', fb.selectionInk),
    accent: token('--color-accent', fb.accent),
    hudBackground: token('--cv-hud-bg', fb.hudBackground),
    hudInk: token('--cv-hud-ink', fb.hudInk),
    warning: token('--cv-warning', fb.warning),
    zone: token('--cv-zone', fb.zone),
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
