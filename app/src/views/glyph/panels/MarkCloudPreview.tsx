import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { DocId, GlyphId, LayerOutline } from '../../../api'
import { Select } from '../../../components/Select'

import { markCloudViewBox, matchMarkAttachment } from './markCloudGeometry'
import styles from './markCloudPreview.module.css'
import panelStyles from './panels.module.css'
import { MARK_CLOUD_PAGE_SIZE, useMarkCloud } from './useMarkCloud'

/** The currently edited base outline and exact master for mark attachment. */
export interface MarkCloudPreviewProps {
  doc: DocId
  gid: GlyphId
  baseName: string
  master: string
  outline: LayerOutline
  path: string | null
  upem: number
}

/** A bounded, read-only visual comparison of matching combining marks. */
export function MarkCloudPreview({
  doc,
  gid,
  baseName,
  master,
  outline,
  path,
  upem,
}: MarkCloudPreviewProps) {
  const { t } = useTranslation()
  const [page, setPage] = useState(0)
  const [anchorName, setAnchorName] = useState<string | null>(null)
  const anchors = outline.anchors.filter(
    (anchor) =>
      anchor.name.length > 0 &&
      !anchor.name.startsWith('_') &&
      Number.isFinite(anchor.x) &&
      Number.isFinite(anchor.y),
  )
  const anchor =
    anchors.find((candidate) => candidate.name === anchorName) ?? anchors[0]
  const result = useMarkCloud(
    doc,
    gid,
    master,
    page,
    anchor !== undefined && outline.master === master,
    setPage,
  )
  const matches = anchor
    ? result.candidates.flatMap(({ cell, detail }) => {
        const attachment = matchMarkAttachment(anchor, detail, cell, master)
        return attachment ? [{ gid: cell.gid, attachment }] : []
      })
    : []

  if (outline.master !== master)
    return (
      <p className={panelStyles.hint}>{t('panel.anchors.cloud.noMaster')}</p>
    )

  if (!anchor)
    return (
      <p className={panelStyles.hint}>{t('panel.anchors.cloud.noAnchor')}</p>
    )

  return (
    <div className={styles.cloud}>
      <Select
        label={t('panel.anchors.cloud.anchor')}
        value={anchor.name}
        options={anchors.map((candidate) => ({
          value: candidate.name,
          label: candidate.name,
        }))}
        onChange={setAnchorName}
      />
      <p className={panelStyles.hint}>{t('panel.anchors.cloud.hint')}</p>
      {result.loading ? (
        <p role="status" className={panelStyles.hint}>
          {t('panel.anchors.cloud.loading')}
        </p>
      ) : null}
      {result.failed || result.partial ? (
        <div role="alert" className={styles.feedback}>
          <p className={panelStyles.hint}>
            {t(
              result.failed
                ? 'panel.anchors.cloud.error'
                : 'panel.anchors.cloud.partial',
            )}
          </p>
          <button
            type="button"
            className={panelStyles.actionButton}
            onClick={result.retry}
          >
            {t('panel.anchors.cloud.retry')}
          </button>
        </div>
      ) : null}
      {!result.loading && !result.failed && matches.length === 0 ? (
        <p className={panelStyles.hint}>{t('panel.anchors.cloud.noMatches')}</p>
      ) : null}
      <div className={styles.grid}>
        {matches.map(({ gid: markId, attachment }) => (
          <figure key={markId} className={styles.figure}>
            <svg
              role="img"
              aria-label={t('panel.anchors.cloud.preview', {
                base: baseName,
                mark: attachment.name,
                anchor: anchor.name,
              })}
              viewBox={markCloudViewBox(
                outline.bbox,
                outline.width,
                upem,
                attachment,
              )}
              className={styles.preview}
            >
              <g transform="scale(1 -1)">
                <path d={path ?? ''} className={styles.base} />
                <g
                  data-mark={attachment.name}
                  transform={`translate(${attachment.dx} ${attachment.dy})`}
                >
                  <path d={attachment.path} className={styles.mark} />
                </g>
                <circle
                  cx={anchor.x}
                  cy={anchor.y}
                  r={Math.max(1, upem * 0.012)}
                  className={styles.anchor}
                />
              </g>
            </svg>
            <figcaption title={attachment.name} className={styles.name}>
              {attachment.name}
            </figcaption>
          </figure>
        ))}
      </div>
      {!result.loading && !result.failed && result.total > 0 ? (
        <p className={panelStyles.hint}>
          {t('panel.anchors.cloud.page', {
            first: result.count > 0 ? page * MARK_CLOUD_PAGE_SIZE + 1 : 0,
            last: page * MARK_CLOUD_PAGE_SIZE + result.count,
            total: result.total,
          })}
        </p>
      ) : null}
      <div className={panelStyles.actions}>
        <button
          type="button"
          className={panelStyles.actionButton}
          disabled={page === 0 || result.loading}
          onClick={() => setPage((value) => value - 1)}
        >
          {t('panel.anchors.cloud.previous')}
        </button>
        <button
          type="button"
          className={panelStyles.actionButton}
          disabled={
            result.loading ||
            result.failed ||
            (page + 1) * MARK_CLOUD_PAGE_SIZE >= result.total
          }
          onClick={() => setPage((value) => value + 1)}
        >
          {t('panel.anchors.cloud.next')}
        </button>
      </div>
    </div>
  )
}
