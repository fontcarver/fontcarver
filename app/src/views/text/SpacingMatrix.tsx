import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { Command, DocId, SpacingRow } from '../../api'
import { documentEditingSuspended } from '../../editor/documentEditParticipants'
import { reportError } from '../../shell/commands'
import { commitSpacingEdit } from '../glyph/panels/kerningGroupEdits'
import {
  clearEffectiveMetricsKey,
  metricsKeyCommand,
} from '../glyph/panels/metricsKeyCommands'
import { spacingLockedForDocument, useSpacingLocked } from './kerningModes'
import {
  displayedKey,
  spacingCellIntent,
  type SpacingField,
} from './spacingMatrix'
import { SpacingMatrixRow } from './SpacingMatrixRow'
import type { useSpacingOverlay } from './useSpacingOverlay'
import styles from './SpacingMatrix.module.css'

/**
 * The Spacing panel: an editable metrics matrix over the glyphs of the run,
 * plus the beam and cut-off display controls.
 *
 * Every cell accepts a number **or** a metrics formula, so spacing and its
 * formulas are authored together instead of one glyph at a time in the
 * inspector. A row whose stored metrics disagree with its formulas offers its
 * own Update button, which runs the same command as the inspector and the Glyph
 * menu. The beam and the cut-off band are display modes of the canvas; their
 * measurements are the rows' own, so the table and the canvas cannot disagree.
 */
export function SpacingMatrix({
  doc,
  master,
  unitsPerEm,
  overlay,
}: {
  doc: DocId
  master: string | null
  unitsPerEm: number
  overlay: ReturnType<typeof useSpacingOverlay>
}) {
  const { t } = useTranslation()
  const [busy, setBusy] = useState(false)
  const [failed, setFailed] = useState(false)
  const locked = useSpacingLocked(doc)
  const { view, rows, onViewChange, refresh } = overlay
  const run = (command: Command, label: string) => {
    if (busy || locked || spacingLockedForDocument(doc)) return
    if (documentEditingSuspended(doc)) return
    setBusy(true)
    setFailed(false)
    void commitSpacingEdit(doc, command, label)
      .catch((error: unknown) => {
        setFailed(true)
        reportError(error)
      })
      .finally(() => {
        setBusy(false)
        refresh()
      })
  }

  const commitCell = (row: SpacingRow, field: SpacingField, raw: string) => {
    const layer = row.layer
    if (layer === null) return
    const intent = spacingCellIntent(row, field, raw)
    if (intent.kind === 'unchanged') return
    const keyCommand = (value: string): Command =>
      metricsKeyCommand(
        row.glyph,
        layer,
        row.glyph_keys,
        row.layer_keys,
        field,
        value,
      )
    // Each cell shows one *effective* formula, so clearing it has to leave no
    // formula behind in either scope: clearing only the layer override would
    // hand the cell straight back to the glyph formula it was replacing.
    const clear = () =>
      clearEffectiveMetricsKey(
        row.glyph,
        layer,
        row.glyph_keys,
        row.layer_keys,
        field,
      )
    if (intent.kind === 'clear') {
      const command = clear()
      if (command) run(command, t('spacingMatrix.undoKey'))
      return
    }
    if (intent.kind === 'formula') {
      run(keyCommand(intent.key), t('spacingMatrix.undoKey'))
      return
    }
    const metrics: Command = {
      SetMetrics: {
        glyph: row.glyph,
        layer,
        width: field === 'width' ? intent.value : null,
        lsb: field === 'left' ? intent.value : null,
        rsb: field === 'right' ? intent.value : null,
      },
    }
    // A typed number replaces every formula for the same metric, so a cell can
    // never leave its own row permanently out of sync.
    const cleared = displayedKey(row, field) === null ? null : clear()
    run(
      cleared ? { Batch: [cleared, metrics] } : metrics,
      t('spacingMatrix.undoMetrics'),
    )
  }

  // "Update" means *this* master. Without a selected master the kernel would
  // read `null` as "every master layer", so the row refuses instead.
  const updateRow = (row: SpacingRow) => {
    if (master === null) return
    run(
      { UpdateMetrics: { glyph: row.glyph, master } },
      t('panel.metrics.update'),
    )
  }

  const numberField = (
    key: 'beamHeight' | 'cutOffFrom' | 'cutOffTo',
    labelKey: string,
  ) => (
    <label className={styles.number}>
      {t(labelKey)}
      <input
        type="number"
        step={1}
        value={view[key]}
        aria-label={t(labelKey)}
        onChange={(event) => {
          const value = Number(event.currentTarget.value)
          if (Number.isFinite(value)) onViewChange({ [key]: value })
        }}
      />
    </label>
  )

  return (
    <details
      className={styles.panel}
      open={view.open}
      onToggle={(event) => onViewChange({ open: event.currentTarget.open })}
    >
      <summary>{t('spacingMatrix.title')}</summary>
      {view.open && (
        <div className={styles.body}>
          <div className={styles.controls}>
            <label>
              <input
                type="checkbox"
                checked={view.beam}
                onChange={(event) =>
                  onViewChange({ beam: event.currentTarget.checked })
                }
              />
              {t('spacingMatrix.beam')}
            </label>
            {view.beam && (
              <>
                {numberField('beamHeight', 'spacingMatrix.beamHeight')}
                <input
                  className={styles.slider}
                  type="range"
                  min={-Math.round(unitsPerEm * 0.5)}
                  max={Math.round(unitsPerEm * 1.2)}
                  step={1}
                  value={view.beamHeight}
                  aria-label={t('spacingMatrix.beamDrag')}
                  title={t('spacingMatrix.beamDrag')}
                  onChange={(event) =>
                    onViewChange({
                      beamHeight: Number(event.currentTarget.value),
                    })
                  }
                />
              </>
            )}
            <label>
              <input
                type="checkbox"
                checked={view.cutOff}
                onChange={(event) =>
                  onViewChange({ cutOff: event.currentTarget.checked })
                }
              />
              {t('spacingMatrix.cutOff')}
            </label>
            {view.cutOff && (
              <>
                {numberField('cutOffFrom', 'spacingMatrix.cutOffFrom')}
                {numberField('cutOffTo', 'spacingMatrix.cutOffTo')}
              </>
            )}
            <button type="button" disabled={busy} onClick={refresh}>
              {t('spacingMatrix.refresh')}
            </button>
          </div>
          <p className={styles.hint}>{t('spacingMatrix.help')}</p>
          {locked && <p role="status">{t('kerningModes.spacingLocked')}</p>}
          {overlay.glyphs.length === 0 && (
            <p role="status">{t('spacingMatrix.empty')}</p>
          )}
          {overlay.loading && <p role="status">{t('spacingMatrix.loading')}</p>}
          {(overlay.failed || failed) && (
            <p role="alert">{t('spacingMatrix.failed')}</p>
          )}
          {rows && rows.length > 0 && (
            <div className={styles.scroll}>
              <table>
                <caption className={styles.srOnly}>
                  {t('spacingMatrix.title')}
                </caption>
                <thead>
                  <tr>
                    <th scope="col">{t('spacingMatrix.glyph')}</th>
                    <th scope="col">{t('spacingMatrix.left')}</th>
                    <th scope="col">{t('spacingMatrix.right')}</th>
                    <th scope="col">{t('spacingMatrix.width')}</th>
                    {view.beam && (
                      <>
                        <th scope="col">{t('spacingMatrix.beamLeft')}</th>
                        <th scope="col">{t('spacingMatrix.beamRight')}</th>
                      </>
                    )}
                    <th scope="col">{t('spacingMatrix.status')}</th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map((row) => (
                    <SpacingMatrixRow
                      key={row.glyph}
                      row={row}
                      beam={view.beam}
                      disabled={busy || locked}
                      canUpdate={master !== null}
                      onCommit={commitCell}
                      onUpdate={updateRow}
                    />
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}
    </details>
  )
}
