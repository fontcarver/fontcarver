import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { call, type BitmapPixelDetection, type DocId } from '../../api'
import { withSettledDocument } from '../../editor/documentPersistence'
import { reportError } from '../../stores/uiStore'
import styles from './BitmapView.module.css'

export function BitmapReconstruction({
  doc,
  master,
}: {
  doc: DocId
  master: string
}) {
  const { t } = useTranslation()
  const [detection, setDetection] = useState<BitmapPixelDetection | null>(null)
  const [busy, setBusy] = useState(false)
  const [checked, setChecked] = useState(false)
  useEffect(() => {
    let active = true
    void call('bitmap.detect_pixel_grid', { doc, master })
      .then((result) => {
        if (active) {
          setDetection(result)
          setChecked(true)
        }
      })
      .catch(() => {
        if (active) setChecked(true)
      })
    return () => {
      active = false
    }
  }, [doc, master])
  return (
    <details open={Boolean(detection?.ppem)}>
      <summary>{t('bitmapParity.reconstruct')}</summary>
      <p className={styles.hint}>
        {detection?.ppem
          ? t('bitmapParity.detected', {
              ppem: detection.ppem,
              units: detection.units_per_pixel,
              count: detection.evidence_glyphs,
            })
          : checked
            ? t('bitmapParity.noGrid')
            : t('bitmapParity.detecting')}
      </p>
      <div className={styles.toolbar}>
        <button
          disabled={busy}
          onClick={() => {
            setBusy(true)
            void withSettledDocument(doc, () =>
              call('bitmap.detect_pixel_grid', { doc, master }),
            )
              .then((result) => {
                setDetection(result)
                setChecked(true)
              })
              .catch(reportError)
              .finally(() => setBusy(false))
          }}
        >
          {t('bitmapParity.detectAgain')}
        </button>
        <button
          disabled={busy || !detection?.ppem}
          onClick={() => {
            setBusy(true)
            void withSettledDocument(doc, async () => {
              const current = await call('bitmap.detect_pixel_grid', {
                doc,
                master,
              })
              setDetection(current)
              await call('bitmap.reconstruct', {
                doc,
                master,
                expected_generation: current.generation,
                glyphs: [],
              })
            })
              .catch(reportError)
              .finally(() => setBusy(false))
          }}
        >
          {t('bitmapParity.reconstructStrike')}
        </button>
      </div>
    </details>
  )
}
