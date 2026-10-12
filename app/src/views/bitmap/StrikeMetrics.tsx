import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { call, type BitmapStrikeInfo, type DocId } from '../../api'
import { registerDocumentEditParticipant } from '../../editor/documentEditParticipants'
import { withSettledDocument } from '../../editor/documentPersistence'
import { noteDocumentEdit } from '../../editor/editSession'
import { reportError } from '../../stores/uiStore'
import styles from './BitmapView.module.css'

/** Persist focused strike metrics through the same document handover as pixels. */
export function StrikeMetrics({
  doc,
  strike,
}: {
  doc: DocId
  strike: BitmapStrikeInfo
}) {
  const { t } = useTranslation(),
    root = useRef<HTMLDivElement>(null),
    dirty = useRef(false),
    revision = useRef(0)
  const [suspended, setSuspended] = useState(false)
  const { ppem, depth } = strike
  useEffect(
    () =>
      registerDocumentEditParticipant(doc, {
        hasPendingEdits: () => dirty.current,
        setSuspended,
        flush: async () => {
          if (!dirty.current) return
          const ascent =
              root.current?.querySelector<HTMLInputElement>('[name="ascent"]'),
            descent =
              root.current?.querySelector<HTMLInputElement>('[name="descent"]')
          if (!ascent || !descent) return
          if (
            [ascent, descent].some(
              (input) =>
                input.value === '' ||
                !input.validity.valid ||
                !Number.isInteger(Number(input.value)),
            )
          )
            throw new Error('Strike metrics must be integers from 0 to 32767')
          const version = revision.current,
            patch = {
              ascent: Number(ascent.value),
              descent: Number(descent.value),
            }
          const current = await call('bitmap.state', { doc }),
            latest = current.strikes.find(
              (s) => s.ppem === ppem && s.depth === depth,
            )
          if (!latest) throw new Error('Bitmap strike no longer exists')
          await call('bitmap.configure', {
            doc,
            expected_generation: current.generation,
            ...latest,
            ...patch,
          })
          if (version === revision.current) dirty.current = false
        },
      }),
    [doc, ppem, depth],
  )
  useEffect(() => {
    if (dirty.current) return
    for (const field of ['ascent', 'descent'] as const) {
      const input = root.current?.querySelector<HTMLInputElement>(
        `[name="${field}"]`,
      )
      if (input) input.value = String(strike[field])
    }
  }, [strike])
  return (
    <div ref={root} className={styles.metrics}>
      {(['ascent', 'descent'] as const).map((field) => (
        <label key={field}>
          {t(`bitmap.${field}`)}
          <input
            type="number"
            name={field}
            min={0}
            max={32767}
            disabled={suspended}
            defaultValue={strike[field]}
            onChange={() => {
              dirty.current = true
              revision.current++
              noteDocumentEdit(doc)
            }}
            onBlur={() => {
              void withSettledDocument(doc, async () => {}).catch(reportError)
            }}
          />
        </label>
      ))}
    </div>
  )
}
