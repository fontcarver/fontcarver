import { useLayoutEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { call } from '../../../api'
import { NumberField } from '../../../components/NumberField'
import { activeEditor } from '../../../editor/activeEditor'
import {
  afterDocumentEdits,
  documentEditRevision,
} from '../../../editor/editSession'
import { commit } from '../../../editor/pointCommands'
import { reportError } from '../../../shell/commands'
import type { Affine } from '../../../editor/affine'
import { useDocumentStore } from '../../../stores/documentStore'

import {
  applyDraftsCommand,
  componentsSignature,
  draftProblem,
  draftTransform,
  editDraft,
  hasDrafts,
  previewMatrix,
  TRANSFORM_FIELDS,
  type TransformDrafts,
} from './cjkTransforms'
import styles from './CjkTransformGrid.module.css'
import panelStyles from './CjkConstructionPanel.module.css'
import sharedStyles from './panels.module.css'
import type { PanelContext } from './panelContext'

/**
 * The per-component transform grid: position and scale for every component of
 * the active layer, previewed live and applied as one undo entry.
 *
 * Positioning a CJK composite means nudging several radicals until the whole
 * character balances, so the grid stages the numbers instead of sending each
 * keystroke to the kernel: the preview above the fields redraws from the draft,
 * and **Apply** writes every changed component in a single command.
 */
/** A stable empty draft set, so a reload does not change the memo identity. */
const EMPTY_DRAFTS: TransformDrafts = {}

export function CjkTransformGrid({
  target,
  gid,
  scene,
  outline,
  reload,
}: Pick<PanelContext, 'target' | 'gid' | 'scene' | 'outline' | 'reload'>) {
  const { t } = useTranslation()
  const [busy, setBusy] = useState(false)
  const components = outline.components
  const signature = componentsSignature(target, components)
  const identity = JSON.stringify([target.doc, target.glyph, target.layer, gid])
  const current = useRef({ signature, identity, mounted: true, sequence: 0 })
  useLayoutEffect(() => {
    if (
      current.current.signature !== signature ||
      current.current.identity !== identity
    )
      current.current.sequence += 1
    current.current.signature = signature
    current.current.identity = identity
  }, [signature, identity])
  useLayoutEffect(() => {
    const lifetime = current.current
    lifetime.mounted = true
    return () => {
      lifetime.mounted = false
      lifetime.sequence += 1
    }
  }, [])
  // The drafts are stored with the component list they were aimed at. A reload
  // that changed that list therefore reads as "no drafts" without an effect,
  // so numbers can never land on a component the user did not type them for.
  const [staged, setStaged] = useState<{
    signature: string
    drafts: TransformDrafts
  }>({ signature, drafts: {} })
  const drafts = useMemo(
    () => (staged.signature === signature ? staged.drafts : EMPTY_DRAFTS),
    [staged, signature],
  )
  const setDrafts = (next: TransformDrafts) =>
    setStaged({ signature, drafts: next })

  const { upem, descender } = scene.metrics
  const problem = draftProblem(drafts)
  const changed = hasDrafts(drafts)

  const preview = useMemo(
    () =>
      components.map(({ component, path }, index) => ({
        path,
        base: component.base,
        matrix: previewMatrix(
          component.transform as Affine,
          draftTransform(drafts, components, index),
        ),
        changed: drafts[index] !== undefined,
      })),
    [components, drafts],
  )

  const apply = async () => {
    if (problem !== null || busy) return
    const editor = activeEditor()
    const sequence = current.current.sequence
    const isCurrent = () =>
      current.current.mounted &&
      current.current.sequence === sequence &&
      activeEditor() === editor
    setBusy(true)
    try {
      await editor?.settle()
      if (!isCurrent()) return
      await afterDocumentEdits(target.doc, async () => {
        if (!isCurrent()) return
        const revision = documentEditRevision(target.doc)
        const generation =
          useDocumentStore.getState().byDoc[target.doc]?.generation
        const fresh = await call('glyphs.outline', {
          doc: target.doc,
          gid,
          layer: target.layer,
        })
        if (
          !isCurrent() ||
          documentEditRevision(target.doc) !== revision ||
          useDocumentStore.getState().byDoc[target.doc]?.generation !==
            generation ||
          componentsSignature(target, fresh.components) !== signature
        )
          return
        await commit(
          target,
          applyDraftsCommand(target, fresh.components, drafts),
          t('cjkConstruction.undoTransforms'),
        )
        if (isCurrent()) setDrafts({})
      })
    } catch (failure) {
      reportError(failure)
    } finally {
      if (current.current.mounted) setBusy(false)
      if (
        current.current.mounted &&
        current.current.identity === identity &&
        activeEditor() === editor
      )
        reload()
    }
  }

  return (
    <div className={styles.grid}>
      <h4 className={styles.heading}>{t('cjkConstruction.transforms')}</h4>
      {components.length === 0 ? (
        <p className={styles.empty}>{t('glyphInspector.noComponents')}</p>
      ) : (
        <>
          <svg
            className={panelStyles.preview}
            viewBox={`0 ${-(descender + upem)} ${Math.max(1, outline.width)} ${upem}`}
            role="img"
            aria-label={t('cjkConstruction.transformPreview')}
          >
            <g transform="scale(1 -1)">
              {preview.map((entry, index) => (
                <path
                  key={`${index}:${entry.base}`}
                  d={entry.path || undefined}
                  transform={`matrix(${entry.matrix.join(' ')})`}
                  opacity={entry.changed ? 1 : 0.55}
                />
              ))}
            </g>
          </svg>
          <p className={panelStyles.hint}>
            {t('cjkConstruction.transformHint')}
          </p>
          {components.map(({ component }, index) => {
            const transform = draftTransform(drafts, components, index)
            return (
              <fieldset
                className={styles.component}
                key={`${index}:${component.base}`}
                disabled={busy}
              >
                <legend className={styles.name}>
                  {t('cjkConstruction.component', {
                    index: index + 1,
                    base: component.base,
                  })}
                </legend>
                <div className={styles.fields}>
                  {TRANSFORM_FIELDS.map((field) => {
                    const label = t(`cjkConstruction.coefficients.${field.key}`)
                    return (
                      <div className={styles.coefficient} key={field.key}>
                        <span aria-hidden="true" className={styles.caption}>
                          {label}
                        </span>
                        <NumberField
                          label={t('cjkConstruction.transformField', {
                            coefficient: label,
                            index: index + 1,
                            base: component.base,
                          })}
                          value={transform[field.index]}
                          formatValue={String}
                          revision={drafts}
                          step={field.step}
                          invalidHint={t('infoBox.invalid')}
                          onCommit={(value) =>
                            setDrafts(
                              editDraft(
                                drafts,
                                components,
                                index,
                                field,
                                value,
                              ),
                            )
                          }
                        />
                      </div>
                    )
                  })}
                </div>
              </fieldset>
            )
          })}
          {problem !== null && problem !== 'empty' && (
            <p role="alert" className={panelStyles.hint}>
              {t(`cjkConstruction.errors.${problem}`)}
            </p>
          )}
          <div className={sharedStyles.actions}>
            <button
              className={sharedStyles.actionButton}
              type="button"
              disabled={problem !== null || busy}
              onClick={() => {
                void apply()
              }}
            >
              {t('cjkConstruction.applyTransforms')}
            </button>
            <button
              className={sharedStyles.actionButton}
              type="button"
              disabled={!changed || busy}
              onClick={() => setDrafts({})}
            >
              {t('cjkConstruction.revertTransforms')}
            </button>
          </div>
        </>
      )}
    </div>
  )
}
