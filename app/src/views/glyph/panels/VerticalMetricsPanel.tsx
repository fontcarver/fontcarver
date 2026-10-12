import { useLayoutEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { call } from '../../../api'
import { Button } from '../../../components/Button'
import { NumberField } from '../../../components/NumberField'
import { activeEditor } from '../../../editor/activeEditor'
import {
  afterDocumentEdits,
  documentEditRevision,
} from '../../../editor/editSession'
import { useDocumentStore } from '../../../stores/documentStore'
import { commit } from '../../../editor/pointCommands'
import { reportError } from '../../../shell/commands'
import {
  spacingLockedForDocument,
  useSpacingLocked,
} from '../../text/kerningModes'

import type { PanelContext } from './panelContext'
import styles from './panels.module.css'

type VerticalMetricsPanelProps = Pick<
  PanelContext,
  'target' | 'gid' | 'glyph' | 'scene' | 'outline' | 'reload'
>

type Overrides = { advance: number | null; origin: number | null }

/** Vertical spacing measured from the resolved ink bounds in font coordinates. */
export function VerticalMetricsPanel({
  target,
  gid,
  glyph,
  scene,
  outline,
  reload,
}: VerticalMetricsPanelProps) {
  const { t } = useTranslation()
  const [revision, setRevision] = useState(0)
  const locked = useSpacingLocked(target.doc)
  const layer = glyph.layers.find((item) => item.id === target.layer)
  const advance = scene.metrics.verticalWidth ?? scene.metrics.upem
  const origin = scene.metrics.verticalOrigin ?? scene.metrics.ascender
  const bounds = outline.bbox
  const top = bounds ? origin - bounds[3] : 0
  const bottom = bounds ? advance - origin + bounds[1] : 0

  const key = JSON.stringify([target.doc, glyph.name, target.layer, gid])
  const queue = useRef(Promise.resolve())
  const current = useRef({ key, generation: 0, mounted: true })
  useLayoutEffect(() => {
    if (current.current.key !== key) current.current.generation += 1
    current.current.key = key
  }, [key])
  useLayoutEffect(() => {
    const lifetime = current.current
    lifetime.mounted = true
    return () => {
      lifetime.mounted = false
      lifetime.generation += 1
    }
  }, [])

  const write = (
    patch: (
      previous: Overrides,
    ) => Overrides | null | Promise<Overrides | null>,
  ) => {
    if (spacingLockedForDocument(target.doc)) return
    const generation = current.current.generation
    const editor = activeEditor()
    const isCurrent = () =>
      current.current.mounted &&
      current.current.generation === generation &&
      activeEditor() === editor
    // Construct each pair only after the preceding edit succeeds or fails.
    // A rejected advance must never become the basis of a queued origin edit.
    queue.current = queue.current
      .then(async () => {
        if (!isCurrent()) return
        await editor?.settle()
        if (!isCurrent() || spacingLockedForDocument(target.doc)) return
        await afterDocumentEdits(target.doc, async () => {
          if (!isCurrent()) return
          const revision = documentEditRevision(target.doc)
          const generation =
            useDocumentStore.getState().byDoc[target.doc]?.generation
          const detail = await call('glyphs.detail', { doc: target.doc, gid })
          const stored = detail.glyph.layers.find(
            (entry) => entry.id === target.layer,
          )
          if (!stored || !isCurrent()) return
          const next = await patch({
            advance: stored.vertical_width ?? null,
            origin: stored.vertical_origin ?? null,
          })
          if (
            !next ||
            (next.advance === (stored.vertical_width ?? null) &&
              next.origin === (stored.vertical_origin ?? null)) ||
            !isCurrent() ||
            spacingLockedForDocument(target.doc) ||
            documentEditRevision(target.doc) !== revision ||
            useDocumentStore.getState().byDoc[target.doc]?.generation !==
              generation
          )
            return
          await commit(
            target,
            {
              SetVerticalMetrics: {
                glyph: glyph.name,
                layer: target.layer,
                ...next,
              },
            },
            t('variation.verticalMetrics'),
          )
        })
      })
      .catch(reportError)
      .finally(() => {
        if (!isCurrent()) return
        // A rejected edit leaves the metric unchanged. Still reconcile the
        // numeric draft, which otherwise only follows a changed model value.
        setRevision((value) => value + 1)
        reload()
      })
  }

  return (
    <>
      <NumberField
        label={t('glyphView.metric.verticalAdvance')}
        value={advance}
        disabled={locked || !layer}
        invalidHint={t('infoBox.invalid')}
        revision={revision}
        onCommit={(value) =>
          write((previous) => ({ ...previous, advance: value }))
        }
      />
      <NumberField
        label={t('glyphView.metric.verticalOrigin')}
        value={origin}
        disabled={locked || !layer}
        invalidHint={t('infoBox.invalid')}
        revision={revision}
        onCommit={(value) =>
          write((previous) => ({ ...previous, origin: value }))
        }
      />
      <NumberField
        label={t('glyphView.metric.tsb')}
        value={top}
        disabled={locked || !layer || bounds === null}
        invalidHint={t('infoBox.invalid')}
        revision={revision}
        onCommit={(value) => {
          if (bounds)
            write(async (previous) => {
              const fresh = await call('glyphs.outline', {
                doc: target.doc,
                gid,
                layer: target.layer,
              })
              return fresh.bbox
                ? { ...previous, origin: fresh.bbox[3] + value }
                : null
            })
        }}
      />
      <NumberField
        label={t('glyphView.metric.bsb')}
        value={bottom}
        disabled={locked || !layer || bounds === null}
        invalidHint={t('infoBox.invalid')}
        revision={revision}
        onCommit={(value) => {
          if (bounds)
            write(async (previous) => {
              const fresh = await call('glyphs.outline', {
                doc: target.doc,
                gid,
                layer: target.layer,
              })
              if (!fresh.bbox) return null
              let effectiveOrigin = previous.origin
              if (effectiveOrigin === null) {
                const info = await call('doc.font_info', { doc: target.doc })
                const master =
                  info.masters.find((entry) => entry.id === fresh.master) ??
                  info.masters[0]
                effectiveOrigin =
                  master?.metrics.default_vertical_origin ??
                  master?.metrics.ascender ??
                  scene.metrics.ascender
              }
              return {
                ...previous,
                advance: effectiveOrigin - fresh.bbox[1] + value,
              }
            })
        }}
      />
      <p className={styles.hint}>
        {t(
          bounds
            ? 'glyphView.metric.verticalBearingsHint'
            : 'glyphView.metric.emptyVerticalBearings',
        )}
      </p>
      <Button
        disabled={
          locked ||
          !layer ||
          (layer.vertical_width == null && layer.vertical_origin == null)
        }
        onClick={() => write(() => ({ advance: null, origin: null }))}
      >
        {t('variation.useMasterMetrics')}
      </Button>
    </>
  )
}
