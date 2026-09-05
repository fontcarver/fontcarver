import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Button } from '../../components/Button'
import { Dialog } from '../../components/Dialog'
import { NumberField } from '../../components/NumberField'
import type { LayerCurves } from '../../editor/pathGeometry'
import type { LayerTarget } from '../../editor/pointCommands'
import { shapeContour } from '../../editor/shapeContours'
import type { ShapeDialogRequest } from '../../editor/tools/toolTypes'

import { commitField } from './commitField'
import styles from './shapeDialog.module.css'

export interface ShapeDialogProps {
  request: ShapeDialogRequest
  target: LayerTarget
  curves: LayerCurves
  onClose: () => void
  onCommitted: () => void
}

/**
 * The shapes numeric dialog (design spec §9).
 *
 * A click with the Rectangle or Ellipse tool opens it seeded at the click, and
 * Enter creates exactly the contour a drag of the same box would have made —
 * one `AddContour`, one undo entry. The chrome, the focus trap and the
 * Enter/Escape grammar come from the shared `Dialog` primitive.
 */
export function ShapeDialog({
  request,
  target,
  curves,
  onClose,
  onCommitted,
}: ShapeDialogProps) {
  const { t } = useTranslation()
  const [x, setX] = useState(Math.round(request.at.x))
  const [y, setY] = useState(Math.round(request.at.y))
  const [width, setWidth] = useState(100)
  const [height, setHeight] = useState(100)
  // Enter reaches both the dialog's confirm and the focused button's own
  // click; one commit is one contour, so the second call is dropped.
  const committed = useRef(false)

  const title = t(
    request.kind === 'rectangle'
      ? 'dialog.shape.rectangleTitle'
      : 'dialog.shape.ellipseTitle',
  )
  const valid = width > 0 && height > 0

  const create = (): void => {
    if (committed.current || !valid) return
    committed.current = true
    const contour = shapeContour(
      request.kind,
      { x0: x, y0: y, x1: x + width, y1: y + height },
      curves,
    )
    commitField(
      target,
      { AddContour: { glyph: target.glyph, layer: target.layer, contour } },
      t(
        request.kind === 'rectangle'
          ? 'editor.undo.drawRectangle'
          : 'editor.undo.drawEllipse',
      ),
      onCommitted,
    )
    onClose()
  }

  return (
    <Dialog
      title={title}
      size="sm"
      onCancel={onClose}
      onConfirm={create}
      footer={
        <>
          <Button onClick={onClose}>{t('common.cancel')}</Button>
          <Button variant="primary" disabled={!valid} onClick={create}>
            {t('dialog.shape.create')}
          </Button>
        </>
      }
    >
      <div className={styles.fields}>
        <NumberField
          label={t('dialog.shape.x')}
          value={x}
          onCommit={setX}
          invalidHint={t('infoBox.invalid')}
        />
        <NumberField
          label={t('dialog.shape.y')}
          value={y}
          onCommit={setY}
          invalidHint={t('infoBox.invalid')}
        />
        <NumberField
          label={t('dialog.shape.width')}
          value={width}
          onCommit={setWidth}
          invalidHint={t('infoBox.invalid')}
        />
        <NumberField
          label={t('dialog.shape.height')}
          value={height}
          onCommit={setHeight}
          invalidHint={t('infoBox.invalid')}
        />
      </div>
    </Dialog>
  )
}
