/** Draw a layer-local review annotation in one undoable canvas gesture. */
import type { AnnotationKind, LayerAnnotation } from '../../api'
import type { Point2 } from '../../canvas/Viewport'
import { i18n } from '../../i18n'
import { controlModifier } from '../gestures/drag/plan'
import { useEditorStore } from '../../stores/editorStore'
import {
  annotationCommand,
  editorMetadata,
  editorTargetKey,
} from '../editorMetadata'
import { useEditorMetadataStore } from '../editorMetadataStore'
import type { LayerTarget } from '../pointCommands'
import type { ToolId } from './registry'
import {
  NO_OUTCOME,
  NO_TOOL_OVERLAY,
  constrainVector,
  type EditorTool,
  type ToolContext,
  type ToolKey,
  type ToolOutcome,
  type ToolOverlay,
  type ToolPointer,
} from './toolTypes'

export interface AnnotationToolSettings {
  kind: AnnotationKind
  text: string
  annotations: readonly LayerAnnotation[]
  label: string
  locked: boolean
}

/** Reads the active layer only when its owner matches the tool's target. */
export function annotationToolSettings(
  target: LayerTarget,
): AnnotationToolSettings | null {
  const active = useEditorStore.getState().activeGlyph
  if (
    !active ||
    active.doc !== target.doc ||
    active.glyph.name !== target.glyph
  )
    return null
  const layer = active.glyph.layers.find((item) => item.id === target.layer)
  if (!layer) return null
  const state = useEditorMetadataStore.getState()
  return {
    kind: state.annotationKind,
    text: state.annotationText || i18n.t('editorMetadata.defaultText'),
    annotations: editorMetadata(layer.user_data).annotations,
    label: i18n.t('editorMetadata.editAnnotations'),
    locked: active.glyph.locked,
  }
}

export class AnnotationTool implements EditorTool {
  readonly id: ToolId = 'annotation'
  private owner: string | null = null
  private from: Point2 | null = null
  private to: Point2 | null = null
  private settings: AnnotationToolSettings | null = null

  constructor(private readonly readSettings = annotationToolSettings) {}

  get busy(): boolean {
    return this.from !== null
  }

  pointerDown(pointer: ToolPointer, context: ToolContext): ToolOutcome {
    if (pointer.button !== 0 || !context.target) return NO_OUTCOME
    this.owner = editorTargetKey(context.target)
    this.settings = this.readSettings(context.target)
    if (!this.settings || this.settings.locked) return NO_OUTCOME
    this.from = context.snap(
      pointer.font,
      controlModifier(pointer.modifiers),
    ).point
    this.to = this.from
    return { handled: true }
  }

  pointerMove(pointer: ToolPointer, context: ToolContext): ToolOutcome {
    if (!this.from) return NO_OUTCOME
    const point = context.snap(
      pointer.font,
      controlModifier(pointer.modifiers),
    ).point
    this.to = pointer.modifiers.shift
      ? constrainVector(this.from, point)
      : point
    return { handled: true }
  }

  pointerUp(pointer: ToolPointer, context: ToolContext): ToolOutcome {
    if (!this.from || !this.settings || !context.target) return NO_OUTCOME
    if (this.owner !== editorTargetKey(context.target)) {
      this.finish()
      return NO_OUTCOME
    }
    this.pointerMove(pointer, context)
    const annotation = this.annotation()
    const settings = this.settings
    this.finish()
    if (!annotation) return NO_OUTCOME
    // Re-read the list at completion so another annotation is never overwritten.
    const current = this.readSettings(context.target)
    if (!current || current.locked) return NO_OUTCOME
    annotation.id =
      globalThis.crypto?.randomUUID?.() ??
      `annotation-${Date.now()}-${Math.random()}`
    return {
      handled: true,
      apply: {
        command: annotationCommand(context.target, [
          ...current.annotations,
          annotation,
        ]),
        label: settings.label,
      },
    }
  }

  private annotation(): LayerAnnotation | null {
    if (!this.from || !this.to || !this.settings) return null
    return {
      id: 'preview',
      kind: this.settings.kind,
      x: this.from.x,
      y: this.from.y,
      end_x: this.to.x,
      end_y: this.to.y,
      text: this.settings.text,
    }
  }

  keyDown(key: ToolKey): ToolOutcome {
    if (key.key !== 'Escape' || !this.busy) return NO_OUTCOME
    this.finish()
    return { handled: true }
  }

  finish(): ToolOutcome {
    this.owner = null
    this.from = null
    this.to = null
    this.settings = null
    return NO_OUTCOME
  }

  overlay(): ToolOverlay {
    const annotation = this.annotation()
    if (!annotation || !this.from || !this.to) return NO_TOOL_OVERLAY
    return { ...NO_TOOL_OVERLAY, annotations: [annotation] }
  }

  cursor(): string {
    return 'crosshair'
  }
}
