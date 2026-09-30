import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { LayerAnnotation } from '../../../api'
import {
  ANNOTATION_KINDS,
  annotationCommand,
  editorMetadata,
  editorTargetKey,
  selectedContours,
} from '../../../editor/editorMetadata'
import { useEditorMetadataStore } from '../../../editor/editorMetadataStore'
import { useEditorStore } from '../../../stores/editorStore'
import { commitField } from '../commitField'
import type { PanelContext } from './panelContext'
import { PanelSection } from './PanelSection'
import { TextRow } from './TextRow'
import styles from './panels.module.css'
import metadataStyles from './editorMetadata.module.css'

/** Focus/locking controls plus the current layer's editable review annotations. */
export function EditorMetadataPanel({ target, glyph, reload }: PanelContext) {
  const { t } = useTranslation()
  const selection = useEditorStore((state) => state.selection)
  const settings = useEditorMetadataStore()
  const [selected, setSelected] = useState<string | null>(null)
  const layer = glyph.layers.find((item) => item.id === target.layer)
  const metadata = editorMetadata(layer?.user_data)
  const paths = selectedContours(selection)
  const focused = settings.focus?.owner === editorTargetKey(target)
  const annotation = metadata.annotations.find((item) => item.id === selected)
  const update = (annotations: LayerAnnotation[]): void =>
    commitField(
      target,
      annotationCommand(target, annotations),
      t('editorMetadata.editAnnotations'),
      reload,
    )
  const patch = (change: Partial<LayerAnnotation>): void => {
    if (!annotation) return
    update(
      metadata.annotations.map((item) =>
        item.id === annotation.id ? { ...item, ...change } : item,
      ),
    )
  }
  const locks = (contours: number[], locked: boolean): void =>
    commitField(
      target,
      {
        EditorMetadata: {
          PathLocks: {
            glyph: target.glyph,
            layer: target.layer,
            contours,
            locked,
          },
        },
      },
      t('editorMetadata.editLocks'),
      reload,
    )

  return (
    <PanelSection
      id="editor-metadata"
      title={t('editorMetadata.title')}
      count={metadata.annotations.length}
    >
      <div className={styles.actions}>
        <button
          type="button"
          className={styles.actionButton}
          disabled={!paths.length}
          aria-pressed={focused}
          onClick={() => settings.focusPaths(target, paths)}
        >
          {t('editorMetadata.focusPaths')}
        </button>
        <button
          type="button"
          className={styles.actionButton}
          disabled={!focused}
          onClick={settings.clearFocus}
        >
          {t('editorMetadata.clearFocus')}
        </button>
        <button
          type="button"
          className={styles.actionButton}
          disabled={glyph.locked || !paths.length}
          onClick={() => locks(paths, true)}
        >
          {t('editorMetadata.lockPaths')}
        </button>
        <button
          type="button"
          className={styles.actionButton}
          disabled={glyph.locked || !metadata.lockedPaths.size}
          onClick={() =>
            locks(
              layer?.contours.flatMap((item, index) =>
                item.id && metadata.lockedPaths.has(item.id) ? [index] : [],
              ) ?? [],
              false,
            )
          }
        >
          {t('editorMetadata.unlockPaths', {
            count: metadata.lockedPaths.size,
          })}
        </button>
      </div>
      <fieldset disabled={glyph.locked} className={metadataStyles.fields}>
        <label className={styles.row}>
          <span className={styles.rowLabel}>{t('editorMetadata.kind')}</span>
          <select
            className={styles.textInput}
            aria-label={t('editorMetadata.kind')}
            value={settings.annotationKind}
            onChange={(event) => {
              const kind = ANNOTATION_KINDS.find(
                (item) => item === event.currentTarget.value,
              )
              if (kind) settings.setAnnotationKind(kind)
            }}
          >
            {ANNOTATION_KINDS.map((kind) => (
              <option key={kind} value={kind}>
                {t(`editorMetadata.kinds.${kind}`)}
              </option>
            ))}
          </select>
        </label>
        <label className={styles.row}>
          <span className={styles.rowLabel}>{t('editorMetadata.newText')}</span>
          <input
            className={styles.textInput}
            aria-label={t('editorMetadata.newText')}
            value={settings.annotationText}
            onChange={(event) =>
              settings.setAnnotationText(event.currentTarget.value)
            }
            placeholder={t('editorMetadata.defaultText')}
          />
        </label>
        <button
          type="button"
          className={styles.actionButton}
          onClick={() => useEditorStore.getState().setTool('annotation')}
        >
          {t('editorMetadata.draw')}
        </button>
        {metadata.annotations.length > 0 && (
          <label className={styles.row}>
            <span className={styles.rowLabel}>
              {t('editorMetadata.annotation')}
            </span>
            <select
              className={styles.textInput}
              aria-label={t('editorMetadata.annotation')}
              value={annotation?.id ?? ''}
              onChange={(event) => setSelected(event.currentTarget.value)}
            >
              <option value="">{t('editorMetadata.select')}</option>
              {metadata.annotations.map((item, index) => (
                <option key={item.id} value={item.id}>
                  {index + 1}.{' '}
                  {item.text || t(`editorMetadata.kinds.${item.kind}`)}
                </option>
              ))}
            </select>
          </label>
        )}
        {annotation && (
          <>
            <TextRow
              label={t('editorMetadata.text')}
              value={annotation.text}
              multiline
              onCommit={(text) => patch({ text })}
            />
            {(['x', 'y', 'end_x', 'end_y'] as const).map((coordinate) => (
              <TextRow
                key={coordinate}
                label={t(`editorMetadata.coordinates.${coordinate}`)}
                value={String(annotation[coordinate])}
                onCommit={(value) => {
                  if (value.trim() && Number.isFinite(Number(value)))
                    patch({ [coordinate]: Number(value) })
                }}
              />
            ))}
            <button
              type="button"
              className={styles.actionButton}
              onClick={() => {
                update(
                  metadata.annotations.filter(
                    (item) => item.id !== annotation.id,
                  ),
                )
                setSelected(null)
              }}
            >
              {t('editorMetadata.remove')}
            </button>
          </>
        )}
      </fieldset>
    </PanelSection>
  )
}
