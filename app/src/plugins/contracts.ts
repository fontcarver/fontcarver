import type { DocId, GlyphId, MasterId, StyleBatchParams } from '../api'
import type { FontBox } from '../canvas/Viewport'
import type { SceneMetrics } from '../views/glyph/glyphScene'

/** Metadata for trusted code shipped with the application; never a download URL. */
export interface PluginManifest {
  id: string
  version: string
  kind: 'reporter' | 'filter'
  nameKey: string
  descriptionKey: string
}

/** One persisted numeric setting a bundled plugin declares in the curated index. */
export interface PluginSetting {
  key: string
  labelKey: string
  /** A toggle is stored as 0 or 1 so every persisted value is one number. */
  kind: 'number' | 'toggle'
  default: number
  min: number
  max: number
  step: number
}

/** A curated index entry: manifest plus provenance and its settings contract. */
export interface PluginIndexEntry extends PluginManifest {
  author: string
  license: string
  /** Always `bundled`: this host has no remote or filesystem plugin loader. */
  origin: 'bundled'
  /** Repository path of the shipped implementation. */
  source: string
  settings: readonly PluginSetting[]
}

/** Declarative, font-space drawing operations. Plugins never receive a canvas. */
export type PluginOverlay =
  | { kind: 'bounds'; bounds: FontBox }
  | { kind: 'path'; path: string }
  | { kind: 'line'; from: [number, number]; to: [number, number] }

/** Which document and layer a reporter is annotating. */
export interface ReporterDocumentContext {
  doc: DocId
  gid: GlyphId
  glyph: string
  /** Persistent layer identifier of the drawn layer. */
  layer: string
  master: MasterId | null
  /** Resolved metrics of the drawn layer, including em size and zones. */
  metrics: SceneMetrics
}

/** What the editor is currently showing, in font units. */
export interface ReporterViewContext {
  /** CSS pixels per font unit. */
  scale: number
  /** Visible region in font space, y-up. */
  visible: FontBox
}

/** Current visible geometry and context, without mutable document access. */
export interface ReporterInput {
  bounds: FontBox | null
  document: ReporterDocumentContext
  view: ReporterViewContext
  /** Persisted settings of this plugin, already validated against the index. */
  settings: Readonly<Record<string, number>>
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
