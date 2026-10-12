import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { Affine } from '../../../editor/affine'
import { editorTargetKey } from '../../../editor/editorMetadata'
import type { LayerTarget } from '../../../editor/pointCommands'
import { ImagePlacementFields } from './ImagePlacementFields'
import { useImagePlacement } from './imagePlacementSession'
import { ImageTraceFields, validTraceSettings } from './ImageTraceFields'
import { ImageTracePreview } from './ImageTracePreview'
import {
  DEFAULT_TRACE_SETTINGS,
  useImageWorkflow,
  type ImageTraceSettings,
} from './useImageWorkflow'
import styles from './imagePanel.module.css'
import panel from './panels.module.css'

const MAX_PNG_BYTES = 16 * 1024 * 1024

/** Imports, places, locks, relinks and traces one PNG source image. */
export function ImagePanel({
  target,
  glyphLocked,
  reload,
}: {
  target: LayerTarget
  glyphLocked: boolean
  reload: () => void
}) {
  const { t } = useTranslation()
  const workflow = useImageWorkflow(target, reload)
  const { state, busy, preview } = workflow
  const image = state?.image ?? null
  const [path, setPath] = useState('')
  const [embed, setEmbed] = useState(false)
  const [settings, setSettings] = useState<ImageTraceSettings>(
    DEFAULT_TRACE_SETTINGS,
  )
  const [fileError, setFileError] = useState<string | null>(null)
  const [onCanvas, setOnCanvas] = useState(false)
  const blocked = busy || workflow.editing || glyphLocked || !state
  const placementBlocked = blocked || Boolean(image?.locked)
  const owner = editorTargetKey(target)
  const openPlacement = useImagePlacement((store) => store.open)
  const closePlacement = useImagePlacement((store) => store.close)
  const broken = Boolean(state?.error) && image !== null
  const writeTransform = (transform: Affine) => {
    if (image) workflow.setImage({ ...image, transform })
  }

  // The canvas owns the handles; this section owns the revision and the write.
  useEffect(() => {
    if (!onCanvas || !image || !state) {
      closePlacement(owner)
      return
    }
    openPlacement({
      owner,
      locked: placementBlocked,
      commit: writeTransform,
    })
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [onCanvas, owner, image, state, placementBlocked, closePlacement])
  useEffect(() => () => closePlacement(owner), [closePlacement, owner])

  const changeSettings = (next: Partial<ImageTraceSettings>) => {
    setSettings((value) => ({ ...value, ...next }))
    workflow.clearPreview()
  }
  const upload = (file: File) => {
    setFileError(null)
    if (file.size > MAX_PNG_BYTES) {
      setFileError(t('imageWorkflow.tooLarge'))
      return
    }
    if (!/\.png$/i.test(file.name)) {
      setFileError(t('imageWorkflow.pngOnly'))
      return
    }
    workflow.importFile(file)
  }
  const relink = (from: string) =>
    workflow.importImage({
      path: from,
      data: null,
      file_name: from.split(/[\\/]/).at(-1) ?? 'image.png',
      embed: false,
    })
  return (
    <>
      <p className={styles.hint}>{t('imageWorkflow.pngHelp')}</p>
      {glyphLocked && (
        <p className={styles.hint}>{t('imageWorkflow.glyphLocked')}</p>
      )}
      {workflow.loading && (
        <p role="status" className={styles.hint}>
          {t('imageWorkflow.loading')}
        </p>
      )}
      {(workflow.error || fileError) && (
        <p role="alert" className={styles.error}>
          {workflow.error || fileError}
        </p>
      )}
      {image && (
        <>
          <p className={styles.hint}>{image.file_name}</p>
          <p className={styles.hint}>
            {state?.png
              ? t(
                  image.data
                    ? 'imageWorkflow.embeddedStatus'
                    : 'imageWorkflow.linkedStatus',
                  { width: state.width, height: state.height },
                )
              : t('imageWorkflow.unresolved')}
          </p>
        </>
      )}
      {broken && (
        <fieldset className={styles.broken} disabled={placementBlocked}>
          <legend>{t('imageWorkflow.brokenLink')}</legend>
          <p role="alert" className={styles.error}>
            {t('imageWorkflow.brokenHelp')}
          </p>
          <p className={styles.hint}>{state?.error}</p>
          {state?.source_path && (
            <p className={styles.hint}>
              {t('imageWorkflow.triedPath', { path: state.source_path })}
            </p>
          )}
          <div className={styles.steps}>
            <button
              type="button"
              className={panel.actionButton}
              onClick={() => {
                workflow.refresh()
                reload()
              }}
            >
              {t('imageWorkflow.retryLink')}
            </button>
            {state?.source_path && (
              <button
                type="button"
                className={panel.actionButton}
                onClick={() => relink(state.source_path as string)}
              >
                {t('imageWorkflow.relinkSame')}
              </button>
            )}
            <button
              type="button"
              className={panel.actionButton}
              disabled={!path.trim()}
              onClick={() => relink(path.trim())}
            >
              {t('imageWorkflow.relinkPath')}
            </button>
          </div>
        </fieldset>
      )}
      <button
        type="button"
        className={panel.actionButton}
        disabled={busy}
        onClick={() => {
          workflow.refresh()
          reload()
        }}
      >
        {t('imageWorkflow.refresh')}
      </button>
      <fieldset className={styles.group} disabled={placementBlocked}>
        <legend>{t('imageWorkflow.import')}</legend>
        <label className={styles.field}>
          {t('imageWorkflow.upload')}
          <input
            className={styles.file}
            type="file"
            accept="image/png,.png"
            onChange={(event) => {
              const file = event.currentTarget.files?.[0]
              event.currentTarget.value = ''
              if (file) void upload(file)
            }}
          />
        </label>
        <label className={styles.field}>
          {t('imageWorkflow.path')}
          <input
            className={panel.textInput}
            value={path}
            onChange={(event) => setPath(event.target.value)}
          />
        </label>
        <label>
          <input
            type="checkbox"
            checked={embed}
            onChange={(event) => setEmbed(event.target.checked)}
          />
          {t('imageWorkflow.embedPath')}
        </label>
        <p className={styles.hint}>{t('imageWorkflow.pathHelp')}</p>
        <button
          type="button"
          className={panel.actionButton}
          disabled={!path.trim()}
          onClick={() =>
            workflow.importImage({
              path: path.trim(),
              data: null,
              file_name: path.trim().split(/[\\/]/).at(-1) ?? 'image.png',
              embed,
            })
          }
        >
          {t('imageWorkflow.importPath')}
        </button>
      </fieldset>
      {image && (
        <>
          <label className={styles.hint}>
            <input
              type="checkbox"
              checked={image.locked}
              disabled={blocked}
              onChange={(event) =>
                workflow.setImage({ ...image, locked: event.target.checked })
              }
            />
            {t('imageWorkflow.lock')}
          </label>
          <fieldset className={styles.group} disabled={placementBlocked}>
            <legend>{t('imageWorkflow.placement')}</legend>
            <label className={styles.hint}>
              <input
                type="checkbox"
                checked={onCanvas}
                onChange={(event) => setOnCanvas(event.target.checked)}
              />
              {t('imageWorkflow.onCanvas')}
            </label>
            <p className={styles.hint}>{t('imageWorkflow.onCanvasHelp')}</p>
            <ImagePlacementFields
              image={image}
              size={
                state?.width && state?.height
                  ? { width: state.width, height: state.height }
                  : null
              }
              disabled={placementBlocked}
              onTransform={writeTransform}
            />
            <button
              type="button"
              className={panel.actionButton}
              onClick={() => workflow.setImage(null)}
            >
              {t('imageWorkflow.remove')}
            </button>
          </fieldset>
          <fieldset
            className={styles.group}
            disabled={busy || workflow.editing || !state?.png}
          >
            <legend>{t('imageWorkflow.trace')}</legend>
            <ImageTraceFields settings={settings} onChange={changeSettings} />
            <button
              type="button"
              className={panel.actionButton}
              disabled={!validTraceSettings(settings)}
              onClick={() => workflow.previewTrace(settings)}
            >
              {t('imageWorkflow.preview')}
            </button>
            <button
              type="button"
              className={panel.actionButton}
              disabled={glyphLocked || !preview}
              onClick={workflow.applyTrace}
            >
              {t('imageWorkflow.apply')}
            </button>
          </fieldset>
          {preview && <ImageTracePreview result={preview.result} />}
        </>
      )}
    </>
  )
}
