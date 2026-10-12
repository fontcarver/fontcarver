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
  const latest = useRef({
    x: Math.round(request.at.x),
    y: Math.round(request.at.y),
    width: 100,
    height: 100,
  })
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
    const values = latest.current
    if (committed.current || values.width <= 0 || values.height <= 0) return
    committed.current = true
    const contour = shapeContour(
      request.kind,
      {
        x0: values.x,
        y0: values.y,
        x1: values.x + values.width,
        y1: values.y + values.height,
      },
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
          onCommit={(value) => {
            latest.current.x = value
            setX(value)
          }}
          invalidHint={t('infoBox.invalid')}
        />
        <NumberField
          label={t('dialog.shape.y')}
          value={y}
          onCommit={(value) => {
            latest.current.y = value
            setY(value)
          }}
          invalidHint={t('infoBox.invalid')}
        />
        <NumberField
          label={t('dialog.shape.width')}
          value={width}
          onCommit={(value) => {
            latest.current.width = value
            setWidth(value)
          }}
          invalidHint={t('infoBox.invalid')}
        />
        <NumberField
          label={t('dialog.shape.height')}
          value={height}
          onCommit={(value) => {
            latest.current.height = value
            setHeight(value)
          }}
          invalidHint={t('infoBox.invalid')}
        />
      </div>
    </Dialog>
  )
}
