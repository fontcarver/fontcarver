import type { Command, Layer } from '../../api'
import type { LayerTarget } from '../../editor/pointCommands'
import { selectedIndexes, type Selection } from '../../editor/selection'
import { newLayerId } from './panels/panelCommands'

/** Duplicate every property while giving the copy its own editable custom role. */
export function duplicateCustomLayerCommand(
  glyph: string,
  source: Layer,
  name: string,
): Command {
  return {
    AddLayer: {
      glyph,
      layer: {
        ...structuredClone(source),
        id: newLayerId(),
        kind: { Custom: name },
        name,
      },
    },
  }
}

/** Copy complete selected paths/components, preserving the source and background. */
export function selectionToBackgroundCommand(
  target: LayerTarget,
  master: string,
  selection: Selection,
): Command | null {
  const contours = [
    ...new Set(
      [...selection.values()].flatMap((item) =>
        item.kind === 'node' ||
        item.kind === 'handle' ||
        item.kind === 'segment'
          ? [item.contour]
          : [],
      ),
    ),
  ].sort((a, b) => a - b)
  const components = selectedIndexes(selection, 'component')
  if (contours.length === 0 && components.length === 0) return null
  return {
    CopySelectionToBackground: {
      glyph: target.glyph,
      from: target.layer,
      master,
      contours,
      components,
    },
  }
}

/**
 * Empty a master's background layer without removing it.
 *
 * "Clear" means the background stops showing anything, not that the layer's
 * identity goes away: the next *Copy to background* or *Selection to
 * background* then starts from a clean sheet instead of appending to old ink,
 * and the layer keeps its width and metadata. One batch, one undo entry, and
 * `null` when there is nothing to clear so the command can disable itself.
 */
export function clearBackgroundCommand(
  glyph: string,
  background: Layer | null,
): Command | null {
  if (!background) return null
  const commands: Command[] = []
  if (background.contours.length > 0)
    commands.push({
      DeleteContours: {
        glyph,
        layer: background.id,
        contours: background.contours.map((_, index) => index),
      },
    })
  if (background.components.length > 0)
    commands.push({
      RemoveComponents: {
        glyph,
        layer: background.id,
        indices: background.components.map((_, index) => index),
      },
    })
  if (commands.length === 0) return null
  return commands.length === 1 ? (commands[0] as Command) : { Batch: commands }
}
