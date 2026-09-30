import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Button } from '../../../components/Button'
import { NumberField } from '../../../components/NumberField'
import { Select } from '../../../components/Select'
import { selectedNodeRefs } from '../../../editor/selection'
import { useEditorStore } from '../../../stores/editorStore'
import { commitField } from '../commitField'
import {
  magneticLinkCommand,
  setServantCommand,
  stemGuidesCommand,
} from './constructionCommands'
import { detectStraightStems } from './constructionGeometry'
import type { PanelContext } from './panelContext'
import styles from './panels.module.css'

/** Persistent guide ownership and X/Y interpolation, driven by canvas selection. */
export function NodeLinksPanel({
  target,
  glyph,
  outline,
  reload,
}: PanelContext) {
  const { t } = useTranslation()
  const selection = useEditorStore((state) => state.selection)
  const [guide, setGuide] = useState('0')
  const [width, setWidth] = useState(200)
  const [detect, setDetect] = useState(false)
  const layer = glyph.layers.find((entry) => entry.id === target.layer)
  if (!layer) return null
  const points = selectedNodeRefs(selection)
  const selected = points.length === 1 ? points[0] : null
  const node = selected
    ? layer.contours[selected.contour]?.points[selected.point]
    : null
  const guides = outline.guides
  const guideIndex = Number(guide)
  const axisButtons = (['X', 'Y'] as const).map((axis) => ({
    axis,
    command: selected ? setServantCommand(target, layer, selected, axis) : null,
    existing: node?.id
      ? layer.node_links?.some(
          (link) => link.node.id === node.id && link.axis === axis,
        )
      : false,
  }))
  const stems = detect ? detectStraightStems(layer.contours, width) : []
  const stemCommand = stemGuidesCommand(target, layer, stems, (amount) =>
    t('construction.stemGuide', { width: Math.round(amount * 100) / 100 }),
  )
  const guideLinks = layer.linked_guides ?? []

  return (
    <>
      <p className={styles.hint}>{t('construction.servantHelp')}</p>
      <div className={styles.actions}>
        {axisButtons.map(({ axis, command, existing }) => (
          <Button
            key={axis}
            disabled={existing ? !selected : !command}
            aria-pressed={!!existing}
            onClick={() => {
              const next =
                existing && selected
                  ? ({
                      NodeLinks: {
                        RemoveServant: {
                          glyph: target.glyph,
                          layer: target.layer,
                          node: selected,
                          axis,
                        },
                      },
                    } as const)
                  : command
              if (next)
                commitField(
                  target,
                  next,
                  t('construction.undo.servant'),
                  reload,
                )
            }}
          >
            {t(
              existing
                ? 'construction.removeServant'
                : 'construction.setServant',
              { axis },
            )}
          </Button>
        ))}
      </div>
      <p className={styles.hint}>{t('construction.magnetHelp')}</p>
      <Select
        label={t('construction.targetGuide')}
        value={guide}
        options={guides.map((entry, index) => ({
          value: String(index),
          label:
            entry.name || t('construction.guideNumber', { index: index + 1 }),
        }))}
        disabled={guides.length === 0}
        onChange={setGuide}
      />
      <div className={styles.actions}>
        <Button
          disabled={points.length === 0 || !guides[guideIndex]}
          onClick={() =>
            commitField(
              target,
              magneticLinkCommand(target, guideIndex, points, false),
              t('construction.undo.link'),
              reload,
            )
          }
        >
          {t('construction.link')}
        </Button>
        <Button
          disabled={points.length === 0 || !guides[guideIndex]}
          onClick={() =>
            commitField(
              target,
              magneticLinkCommand(target, guideIndex, points, true),
              t('construction.undo.link'),
              reload,
            )
          }
        >
          {t('construction.snapLink')}
        </Button>
      </div>
      {guideLinks.map((link) => {
        const index = guides.findIndex((entry) => entry.id === link.guide_id)
        return (
          <div className={styles.listRow} key={link.guide_id}>
            <span className={styles.rowValue}>
              {t('construction.linkedCount', {
                guide:
                  guides[index]?.name ||
                  t('construction.guideNumber', { index: index + 1 }),
                count: link.nodes.length,
              })}
            </span>
            <Button
              disabled={index < 0}
              onClick={() =>
                commitField(
                  target,
                  {
                    NodeLinks: {
                      UnlinkGuide: {
                        glyph: target.glyph,
                        layer: target.layer,
                        guide_index: index,
                      },
                    },
                  },
                  t('construction.undo.unlink'),
                  reload,
                )
              }
            >
              {t('construction.unlink')}
            </Button>
          </div>
        )
      })}
      <p className={styles.hint}>{t('construction.stemHelp')}</p>
      <NumberField
        label={t('construction.maxStem')}
        value={width}
        onCommit={(value) => {
          setWidth(value)
          setDetect(false)
        }}
      />
      <div className={styles.actions}>
        <Button
          disabled={!Number.isFinite(width) || width <= 0}
          onClick={() => setDetect(true)}
        >
          {t('construction.detectStems')}
        </Button>
        {detect && (
          <Button
            disabled={!stemCommand}
            onClick={() => {
              if (stemCommand)
                commitField(
                  target,
                  stemCommand,
                  t('construction.undo.stems'),
                  () => {
                    setDetect(false)
                    reload()
                  },
                )
            }}
          >
            {t('construction.addStemGuides')}
          </Button>
        )}
      </div>
      {detect && (
        <p role="status" className={styles.hint}>
          {t('construction.stemCount', { count: stems.length })}
        </p>
      )}
    </>
  )
}
