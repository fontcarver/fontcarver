import type { DocId, GlyphId, MasterId, StyleBatchParams } from '../api'
import type { FontBox } from '../canvas/Viewport'

/** Metadata for trusted code shipped with the application; never a download URL. */
export interface PluginManifest {
  id: string
  version: string
  kind: 'reporter' | 'filter'
  nameKey: string
  descriptionKey: string
}

/** Declarative, font-space drawing operations. Plugins never receive a canvas. */
export type PluginOverlay =
  { kind: 'bounds'; bounds: FontBox } | { kind: 'path'; path: string }

/** Current visible geometry, without mutable document or transport access. */
export interface ReporterInput {
  bounds: FontBox | null
}

/** A reporter computes transient annotations; disabling it removes its drawings. */
export interface ReporterPlugin {
  manifest: PluginManifest & { kind: 'reporter' }
  report: (input: ReporterInput) => readonly PluginOverlay[]
}

/** Stable identity of the editable master layer a filter targets. */
export interface FilterTarget {
  doc: DocId
  gid: GlyphId
  glyph: string
  layer: string
  master: MasterId
}

/** Immutable request used for both preview and apply; the host supplies revision flags. */
export type FilterPlan = Omit<
  StyleBatchParams,
  'dry_run' | 'expected_generation'
>

/** A filter prepares an existing revision-checked kernel operation without executing it. */
export interface FilterPlugin {
  manifest: PluginManifest & { kind: 'filter' }
  plan: (
    target: FilterTarget,
    parameters: Readonly<Record<string, number>>,
    label: string,
  ) => FilterPlan
}

/** The shipped implementations supported by this plugin host. */
export type BundledPlugin = ReporterPlugin | FilterPlugin
