/** Typed access to layer-local editor metadata, independent of font export. */
import type { AnnotationKind, Command, Contour, LayerAnnotation } from '../api'
import type { EditOutline } from './outlineGeometry'
import type { LayerTarget } from './pointCommands'
import type { EditTarget, Selection } from './selection'

export const EDITOR_METADATA_KEY = 'com.fontcarver.editor'
export const ANNOTATION_KINDS: readonly AnnotationKind[] = [
  'Text',
  'Arrow',
  'Circle',
  'Rectangle',
  'Plus',
  'Minus',
]

function record(value: unknown): Record<string, unknown> | null {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null
}

function isAnnotation(value: unknown): value is LayerAnnotation {
  const item = record(value)
  return (
    item !== null &&
    typeof item.id === 'string' &&
    item.id.length > 0 &&
    ANNOTATION_KINDS.includes(item.kind as AnnotationKind) &&
    typeof item.text === 'string' &&
    [item.x, item.y, item.end_x, item.end_y].every(
      (n) => typeof n === 'number' && Number.isFinite(n),
    )
  )
}

/** Ignore corrupt or foreign editor metadata without disturbing other user data. */
export function editorMetadata(userData: unknown): {
  annotations: LayerAnnotation[]
  lockedPaths: ReadonlySet<string>
} {
  const data = record(record(userData)?.[EDITOR_METADATA_KEY])
  return {
    annotations: Array.isArray(data?.annotations)
      ? data.annotations.filter(isAnnotation)
      : [],
    lockedPaths: new Set(
      Array.isArray(data?.locked_paths)
        ? data.locked_paths.filter((id): id is string => typeof id === 'string')
        : [],
    ),
  }
}

/** Stable owner identity for a transient focus session. */
export function editorTargetKey(target: LayerTarget): string {
  return JSON.stringify([target.doc, target.glyph, target.layer])
}

/** The unique path indexes addressed by a canvas selection. */
export function selectedContours(selection: Selection): number[] {
  return [
    ...new Set(
      [...selection.values()].flatMap((target) =>
        'contour' in target ? [target.contour] : [],
      ),
    ),
  ].sort((a, b) => a - b)
}

/** Whether a path may be selected in the current editing scope. */
export function editableContour(
  contours: readonly Contour[],
  index: number,
  locks: ReadonlySet<string>,
  focus: ReadonlySet<number> | null,
): boolean {
  const contour = contours[index]
  return (
    contour !== undefined &&
    (focus === null || focus.has(index)) &&
    (contour.id === null || !locks.has(contour.id))
  )
}

/** Filter indexes while retaining original contour/point references for commands. */
export function editableOutline(
  outline: EditOutline,
  locks: ReadonlySet<string>,
  focus: ReadonlySet<number> | null,
  glyphLocked: boolean,
): EditOutline {
  if (!glyphLocked && locks.size === 0 && focus === null) return outline
  const allowed = (index: number): boolean =>
    !glyphLocked && editableContour(outline.contours, index, locks, focus)
  return {
    contours: outline.contours,
    points: outline.points.filter((point) => allowed(point.ref.contour)),
    segments: outline.segments.filter((segment) => allowed(segment.contour)),
  }
}

/** A focus scope permits only objects on its selected contours. */
export function inPathFocus(
  target: EditTarget,
  focus: ReadonlySet<number> | null,
): boolean {
  return focus === null || ('contour' in target && focus.has(target.contour))
}

/** Replace a layer's annotations as one reversible document edit. */
export function annotationCommand(
  target: LayerTarget,
  annotations: readonly LayerAnnotation[],
): Command {
  return {
    EditorMetadata: {
      Annotations: {
        glyph: target.glyph,
        layer: target.layer,
        annotations: [...annotations],
      },
    },
  }
}
