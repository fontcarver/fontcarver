/**
 * The commands the inspector panels emit.
 *
 * Every control in `views/glyph/panels/` commits exactly one `doc.apply`, so
 * the builders live here as pure functions: a test can assert the command a
 * button sends without rendering the button, and the panels stay declarative.
 * Nothing in this module talks to the kernel — `commitField` does that.
 */

import type {
  Anchor,
  Command,
  Component,
  Guide,
  Layer,
  LayerKind,
  MetricsKeys,
} from '../../../api'
import type { LayerTarget } from '../../../editor/pointCommands'
import type { GuideScope } from '../../../editor/selection'

/** The nine anchor names the Anchors panel suggests (design spec §7.5). */
export const ANCHOR_SUGGESTIONS: readonly string[] = [
  'top',
  'bottom',
  'center',
  'ogonek',
  'entry',
  'exit',
  '_top',
  '_bottom',
  '_center',
]

/**
 * A fresh layer identifier.
 *
 * `Layer.id` is `#[serde(default)]` in the model, so the kernel would mint one
 * for a layer sent without it — but the generated TypeScript type has no
 * optional id, and a layer the interface has to address again (to select it,
 * to delete it) is easier to follow when the client named it. `randomUUID` is
 * the browser's own generator; the fallback covers jsdom and insecure origins.
 */
export function newLayerId(): string {
  const uuid = globalThis.crypto?.randomUUID?.()
  if (uuid) return uuid.replace(/-/g, '')
  return `layer-${Date.now().toString(16)}-${Math.floor(
    Math.random() * 0xffffffff,
  ).toString(16)}`
}

/** An empty layer of one role, with the width of the layer it accompanies. */
export function emptyLayer(
  kind: LayerKind,
  name: string,
  width: number,
): Layer {
  return {
    id: newLayerId(),
    kind,
    name,
    width,
    vertical_width: null,
    vertical_origin: null,
    contours: [],
    components: [],
    anchors: [],
    guides: [],
    background_image: null,
    hints: {
      h_stems: [],
      v_stems: [],
      d_stems: [],
      flex: false,
      hint_masks: [],
    },
    tt_instructions: null,
    color_layers: null,
    user_data: null,
  }
}

/* ------------------------------------------------------------------ Glyph */

/** `RenameGlyph`: the Glyph panel's name field. */
export function renameGlyphCommand(from: string, to: string): Command {
  return { RenameGlyph: { from, to } }
}

/** `SetUnicodes`: the Glyph panel's Unicode chips. */
export function setUnicodesCommand(
  glyph: string,
  unicodes: readonly number[],
): Command {
  return { SetUnicodes: { glyph, unicodes: [...unicodes] } }
}

/** The fields `SetGlyphMeta` can change; anything omitted is left alone. */
export interface GlyphMetaUpdate {
  export?: boolean
  /** `null` clears the colour label; omitting the key leaves it as it is. */
  colorLabel?: number | null
  tags?: readonly string[]
  note?: string
}

/**
 * `SetGlyphMeta`: the export toggle, colour label, tags and note.
 *
 * The command's own fields are all "leave alone" when null, and `color_label`
 * is doubly optional — an absent key leaves it, an explicit `null` clears it —
 * so the builder only writes the key when the caller passed one.
 */
export function setGlyphMetaCommand(
  glyph: string,
  update: GlyphMetaUpdate,
): Command {
  const command = {
    glyph,
    export: update.export ?? null,
    tags: update.tags ? [...update.tags] : null,
    note: update.note ?? null,
  }
  return {
    SetGlyphMeta:
      'colorLabel' in update
        ? { ...command, color_label: update.colorLabel }
        : command,
  }
}

/** `DuplicateGlyph`: the Glyph menu's Duplicate. */
export function duplicateGlyphCommand(source: string, name: string): Command {
  return { DuplicateGlyph: { source, name } }
}

/** A name for the duplicate of `source` that no glyph in `taken` uses. */
export function duplicateName(
  source: string,
  taken: ReadonlySet<string>,
): string {
  for (let index = 1; index < 1000; index += 1) {
    const candidate = index === 1 ? `${source}.copy` : `${source}.copy${index}`
    if (!taken.has(candidate)) return candidate
  }
  return `${source}.copy${Date.now()}`
}

/* ---------------------------------------------------------------- Metrics */

/** `SetMetricsKeys`: the Metrics panel's formula row. */
export function setMetricsKeysCommand(
  glyph: string,
  keys: MetricsKeys,
): Command {
  return { SetMetricsKeys: { glyph, keys } }
}

/** A metrics-key string with empty entries normalised back to `null`. */
export function metricsKeysWith(
  keys: MetricsKeys,
  field: keyof MetricsKeys,
  value: string,
): MetricsKeys {
  const trimmed = value.trim()
  return { ...keys, [field]: trimmed === '' ? null : trimmed }
}

/** Whether any horizontal metrics formula is set. */
export function hasMetricsKeys(keys: MetricsKeys): boolean {
  return keys.left !== null || keys.right !== null || keys.width !== null
}

/* ----------------------------------------------------------------- Layers */

/** `AddLayer`: a new background or backup layer for one master. */
export function addLayerCommand(
  glyph: string,
  kind: LayerKind,
  name: string,
  width: number,
): Command {
  return { AddLayer: { glyph, layer: emptyLayer(kind, name, width) } }
}

/** `RemoveLayer`: the Layers panel's delete. Master layers are never passed. */
export function removeLayerCommand(glyph: string, layer: string): Command {
  return { RemoveLayer: { glyph, layer } }
}

/** `CopyLayer`: copy the edited layer into its background or a backup. */
export function copyLayerCommand(
  glyph: string,
  from: string,
  to: LayerKind,
): Command {
  return { CopyLayer: { glyph, from, to } }
}

/** `SwapWithBackground`: exchange a master layer with its background. */
export function swapWithBackgroundCommand(
  glyph: string,
  master: string,
): Command {
  return { SwapWithBackground: { glyph, master } }
}

/* ------------------------------------------------------------- Components */

/** `AddComponent`: the picker's chosen base, placed at the origin. */
export function addComponentCommand(
  target: LayerTarget,
  base: string,
): Command {
  return {
    AddComponent: {
      glyph: target.glyph,
      layer: target.layer,
      component: newComponent(base),
    },
  }
}

/** A component of `base` with an identity transform and no options set. */
export function newComponent(base: string): Component {
  return {
    base,
    transform: [1, 0, 0, 1, 0, 0],
    anchor: null,
    auto_align: false,
    locked: false,
    use_my_metrics: false,
    round_to_grid: false,
    smart: null,
    point_match: null,
  }
}

/** `RemoveComponents`: the Components panel's delete. */
export function removeComponentsCommand(
  target: LayerTarget,
  indices: readonly number[],
): Command {
  return {
    RemoveComponents: {
      glyph: target.glyph,
      layer: target.layer,
      indices: [...indices],
    },
  }
}

/** `DecomposeComponents`: the panel's decompose, and the Glyph menu's. */
export function decomposeComponentsCommand(
  target: LayerTarget,
  indices: readonly number[],
): Command {
  return {
    DecomposeComponents: {
      glyph: target.glyph,
      layer: target.layer,
      indices: [...indices],
    },
  }
}

/** The list `from` moved to `to`, both indexes into the same array. */
export function reordered<T>(
  items: readonly T[],
  from: number,
  to: number,
): T[] {
  const next = [...items]
  const [moved] = next.splice(from, 1)
  if (moved === undefined) return [...items]
  next.splice(Math.max(0, Math.min(next.length, to)), 0, moved)
  return next
}

/**
 * Reordering components as one undo entry.
 *
 * There is no "move component" command: the list is addressed by index, so the
 * honest way to reorder is to rewrite every entry that moved with
 * `SetComponent`, batched into a single entry. Only the slice between the two
 * positions changes, so a drag of one row over its neighbour writes two.
 */
export function reorderComponentsCommand(
  target: LayerTarget,
  components: readonly Component[],
  from: number,
  to: number,
): Command | null {
  if (from === to || from < 0 || from >= components.length) return null
  const next = reordered(components, from, to)
  const writes: Command[] = []
  next.forEach((component, index) => {
    if (component === components[index]) return
    writes.push({
      SetComponent: {
        glyph: target.glyph,
        layer: target.layer,
        index,
        component,
      },
    })
  })
  if (writes.length === 0) return null
  return writes.length === 1 && writes[0] ? writes[0] : { Batch: writes }
}

/* ---------------------------------------------------------------- Anchors */

/** `AddAnchor`: the Anchors panel's add, at the given position. */
export function addAnchorCommand(target: LayerTarget, anchor: Anchor): Command {
  return { AddAnchor: { glyph: target.glyph, layer: target.layer, anchor } }
}

/** `RemoveAnchors`: the Anchors panel's delete. */
export function removeAnchorsCommand(
  target: LayerTarget,
  indices: readonly number[],
): Command {
  return {
    RemoveAnchors: {
      glyph: target.glyph,
      layer: target.layer,
      indices: [...indices],
    },
  }
}

/** The first suggested anchor name the layer does not already carry. */
export function suggestAnchorName(taken: readonly Anchor[]): string {
  const used = new Set(taken.map((anchor) => anchor.name))
  return (
    ANCHOR_SUGGESTIONS.find((name) => !used.has(name)) ??
    `anchor${taken.length + 1}`
  )
}

/* ----------------------------------------------------------------- Guides */

/** `AddGuide` in the layer's own scope or in the master's. */
export function addGuideCommand(
  target: LayerTarget,
  scope: GuideScope,
  master: string | null,
  guide: Guide,
): Command {
  return scope === 'layer'
    ? {
        AddGuide: {
          glyph: target.glyph,
          layer: target.layer,
          master: null,
          guide,
        },
      }
    : { AddGuide: { glyph: null, layer: null, master, guide } }
}

/** `RemoveGuide` in the layer's own scope or in the master's. */
export function removeGuideCommand(
  target: LayerTarget,
  scope: GuideScope,
  master: string | null,
  index: number,
): Command {
  return scope === 'layer'
    ? {
        RemoveGuide: {
          glyph: target.glyph,
          layer: target.layer,
          master: null,
          index,
        },
      }
    : { RemoveGuide: { glyph: null, layer: null, master, index } }
}

/** A guide through one point at one angle, unnamed and unlocked. */
export function newGuide(x: number, y: number, angle: number): Guide {
  return {
    x,
    y,
    angle,
    name: null,
    locked: false,
    color: null,
    anchor: 'Origin',
  }
}
