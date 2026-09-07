import type { Glyph } from '../../api'
import type { TextTabState } from '../../stores/uiStore'
import type { PositionedGlyph } from './textLayout'

/** A Unicode-safe title, without splitting supplementary characters. */
export function textTitle(text: string): string {
  return Array.from(text).slice(0, 24).join('')
}

/** Initial legacy glyph text; called only before the user supplies text. */
export function glyphText(glyph: Pick<Glyph, 'name' | 'unicodes'>): string {
  const cp = glyph.unicodes[0]
  return cp === undefined ? `/${glyph.name}` : String.fromCodePoint(cp)
}

export function initialTextState(
  master: string | null,
  text = '',
): TextTabState {
  return {
    text,
    activeIndex: 0,
    mode: 'edit',
    master,
    size: 128,
    features: [
      ['kern', true],
      ['liga', true],
      ['calt', true],
    ],
    direction: 'ltr',
    previewTheme: 'light',
    previewCollapsed: false,
  }
}

/** Preserve the same source occurrence, including repeated glyphs and RTL. */
export function reconcileTextIndex(
  previous: PositionedGlyph | undefined,
  glyphs: readonly PositionedGlyph[],
): number | null {
  if (!previous) return null
  const index = glyphs.findIndex(
    (glyph) =>
      glyph.sourceByte === previous.sourceByte &&
      glyph.name === previous.name &&
      glyph.documentGid === previous.documentGid,
  )
  return index < 0 ? null : index
}
