import type { Ref } from 'react'

/** One selectable design-space master in text presentation controls. */
export interface TextMasterOption {
  id: string
  name: string
}

/** An OpenType feature tag paired with its enabled state. */
export type TextFeature = [tag: string, enabled: boolean]

/** Directions available to the text shaper. */
export type TextDirection = 'ltr' | 'rtl' | 'ttb'

/** Sample-menu actions. All-encoded resolution remains the caller's job. */
export type TextSampleId = 'latin' | 'chinese' | 'all-encoded'

/** Controlled properties for the text editing bar. */
export interface TextBarProps {
  text: string
  onTextChange: (text: string) => void
  inputRef?: Ref<HTMLInputElement>
  size: number
  onSizeChange: (size: number) => void
  masters: TextMasterOption[]
  master: string
  onMasterChange: (master: string) => void
  features: TextFeature[]
  onFeaturesChange: (features: TextFeature[]) => void
  direction: TextDirection
  onDirectionChange: (direction: TextDirection) => void
  onSample: (sample: TextSampleId) => void
}

/** One already-positioned glyph path, expressed entirely in font units. */
export interface PreviewGlyph {
  path: string
  x: number
  y: number
  xAdvance: number
  yAdvance: number
}

export type PreviewStatus = 'ready' | 'stale' | 'compile-failed'
export type PreviewTheme = 'light' | 'dark'

/** Controlled properties for the outline-based text preview strip. */
export interface PreviewStripProps {
  glyphs: PreviewGlyph[]
  unitsPerEm: number
  status: PreviewStatus
  onDismissStatus: () => void
  theme: PreviewTheme
  onThemeChange: (theme: PreviewTheme) => void
  collapsed: boolean
  onCollapsedChange: (collapsed: boolean) => void
}
