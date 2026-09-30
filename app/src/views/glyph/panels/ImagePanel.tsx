import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { Image } from '../../../api'
import type { LayerTarget } from '../../../editor/pointCommands'
import { ImageTracePreview } from './ImageTracePreview'
import { useImageWorkflow, type ImageTraceSettings } from './useImageWorkflow'
import styles from './imagePanel.module.css'
import panel from './panels.module.css'

const IDENTITY: Image['transform'] = [1, 0, 0, 1, 0, 0]
const MAX_PNG_BYTES = 16 * 1024 * 1024
const FIELDS = [
  'scaleX',
  'shearY',
  'shearX',
  'scaleY',
  'translateX',
  'translateY',
] as const

/** Imports, places, locks, and traces one PNG source image without discarding it. */
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
  const [draft, setDraft] = useState<{
    source: string
    values: string[]
  } | null>(null)
  const [path, setPath] = useState('')
  const [embed, setEmbed] = useState(false)
  const [settings, setSettings] = useState<ImageTraceSettings>({
    threshold: 128,
    tolerance: 1,
    append: true,
  })
  const [fileError, setFileError] = useState<string | null>(null)
  const transformKey = JSON.stringify(image?.transform ?? IDENTITY)
  const placement =
    draft?.source === transformKey
      ? draft.values
      : (image?.transform ?? IDENTITY).map(String)
  const blocked = busy || workflow.editing || glyphLocked || !state
  const placementBlocked = blocked || Boolean(image?.locked)
  const transform = placement.map((value) =>
    value.trim() === '' ? NaN : Number(value),
  ) as Image['transform']
  const validTransform = transform.every(Number.isFinite)
  const validTrace =
    Number.isInteger(settings.threshold) &&
    settings.threshold >= 1 &&
    settings.threshold <= 255 &&
    Number.isFinite(settings.tolerance) &&
    settings.tolerance >= 0 &&
    settings.tolerance <= 16
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
      {state?.error && (
        <p role="alert" className={styles.error}>
          {state.error}
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
            <div className={styles.matrix}>
              {FIELDS.map((field, index) => (
                <label key={field} className={styles.field}>
                  {t(`imageWorkflow.${field}`)}
                  <input
                    type="number"
                    step="any"
                    className={panel.textInput}
                    value={placement[index]}
                    onChange={(event) => {
                      const value = event.target.value
                      setDraft({
                        source: transformKey,
                        values: placement.map((old, i) =>
                          i === index ? value : old,
                        ),
                      })
                      workflow.clearPreview()
                    }}
                  />
                </label>
              ))}
            </div>
            <p className={styles.hint}>{t('imageWorkflow.affineHelp')}</p>
            <button
              type="button"
              className={panel.actionButton}
              disabled={!validTransform}
              onClick={() => workflow.setImage({ ...image, transform })}
            >
              {t('imageWorkflow.applyPlacement')}
            </button>
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
            <p className={styles.hint}>{t('imageWorkflow.traceHelp')}</p>
            <label className={styles.field}>
              {t('imageWorkflow.threshold')}
              <input
                type="number"
                min={1}
                max={255}
                step={1}
                className={panel.textInput}
                value={
                  Number.isFinite(settings.threshold) ? settings.threshold : ''
                }
                onChange={(event) =>
                  changeSettings({ threshold: event.target.valueAsNumber })
                }
              />
            </label>
            <label className={styles.field}>
              {t('imageWorkflow.tolerance')}
              <input
                type="number"
                min={0}
                max={16}
                step={0.1}
                className={panel.textInput}
                value={
                  Number.isFinite(settings.tolerance) ? settings.tolerance : ''
                }
                onChange={(event) =>
                  changeSettings({ tolerance: event.target.valueAsNumber })
                }
              />
            </label>
            <label>
              <input
                type="checkbox"
                checked={settings.append}
                onChange={(event) =>
                  changeSettings({ append: event.target.checked })
                }
              />
              {t('imageWorkflow.append')}
            </label>
            {!settings.append && (
              <p className={styles.hint}>{t('imageWorkflow.replaceHelp')}</p>
            )}
            <button
              type="button"
              className={panel.actionButton}
              disabled={!validTrace}
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
