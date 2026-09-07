/**
 * What every inspector panel is handed.
 *
 * The panels live in the shell's inspector, which is a sibling of the glyph
 * view, so the data comes through the editor store: the mounted view publishes
 * the glyph, the editable layer and the reload callback, and the panels read
 * that one record. No panel keeps a copy of document state — every field reads
 * the layer and writes one command.
 */

import type {
  Glyph,
  GlyphCell,
  GlyphId,
  Guide,
  LayerOutline,
} from '../../../api'
import type { LayerTarget } from '../../../editor/pointCommands'
import type { GlyphScene } from '../glyphScene'

/** The glyph, the layer and the target every panel command names. */
export interface PanelContext {
  target: LayerTarget
  /** Session handle of the glyph, for the calls that take one. */
  gid: GlyphId
  glyph: Glyph
  /** The grid cell, which carries the categories `fc-glyphdata` derived. */
  cell: GlyphCell | null
  scene: GlyphScene
  outline: LayerOutline
  master: string | null
  masterGuides: readonly Guide[]
  /** Refetches the layer after a command; the view's own reload. */
  reload: () => void
}
