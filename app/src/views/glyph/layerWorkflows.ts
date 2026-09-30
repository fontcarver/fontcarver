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
