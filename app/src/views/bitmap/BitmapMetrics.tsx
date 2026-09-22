import { useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import type { BitmapGlyph } from '../../api'
import { registerDocumentEditParticipant } from '../../editor/documentEditParticipants'
import { noteDocumentEdit } from '../../editor/editSession'
import { reportError } from '../../stores/uiStore'
import { BitmapSession } from './bitmapSession'
import { resize } from './pixels'
import styles from './BitmapView.module.css'
const fields = [
  'width',
  'height',
  'origin_x',
  'origin_y',
  'advance',
  'verticalAdvance',
  'verticalX',
  'verticalY',
] as const
type Field = (typeof fields)[number]
function values(g: BitmapGlyph): Record<Field, number | string> {
  return {
    ...g.bitmap,
    advance: g.advance,
    verticalAdvance: g.vertical_advance ?? '',
    verticalX: g.vertical_origin?.[0] ?? '',
    verticalY: g.vertical_origin?.[1] ?? '',
  }
}
/** Numeric drafts participate in Save/Undo even when the input still has focus. */
export function BitmapMetrics({
  session,
  value,
  disabled,
}: {
  session: BitmapSession
  value: BitmapGlyph
  disabled: boolean
}) {
  const { t } = useTranslation(),
    root = useRef<HTMLDivElement>(null),
    dirty = useRef(false)
  const commit = async () => {
    if (!dirty.current) return
    if (session.snapshot().busy || session.snapshot().suspended)
      throw new Error(
        'Wait for the current bitmap operation before committing metrics',
      )
    const previous = session.snapshot().value
    if (!previous) return
    const data = values(previous)
    for (const field of fields) {
      const input = root.current?.querySelector<HTMLInputElement>(
        `[name="${field}"]`,
      )
      if (!input) continue
      const raw = input.value.trim(),
        nullable = field.startsWith('vertical')
      if (
        (!raw && !nullable) ||
        !input.validity.valid ||
        (raw && !Number.isInteger(Number(raw)))
      )
        throw new Error(
          'Bitmap metrics must be integers within the displayed bounds',
        )
      data[field] = raw === '' ? '' : Number(raw)
    }
    let next = resize(previous, Number(data.width), Number(data.height))
    next = {
      ...next,
      advance: Number(data.advance),
      vertical_advance:
        data.verticalAdvance === '' ? null : Number(data.verticalAdvance),
    }
    next.bitmap.origin_x = Number(data.origin_x)
    next.bitmap.origin_y = Number(data.origin_y)
    if ((data.verticalX === '') !== (data.verticalY === ''))
      throw new Error('Set both vertical origin coordinates, or clear both')
    next.vertical_origin =
      data.verticalX === ''
        ? undefined
        : [Number(data.verticalX), Number(data.verticalY)]
    dirty.current = false
    if (JSON.stringify(next) !== JSON.stringify(previous)) session.edit(next)
    await session.flush()
  }
  const latest = useRef(commit)
  useEffect(() => {
    latest.current = commit
  })
  useEffect(
    () =>
      registerDocumentEditParticipant(session.target.doc, {
        flush: () => latest.current(),
        hasPendingEdits: () => dirty.current,
      }),
    [session],
  )
  useEffect(() => {
    if (dirty.current) return
    const data = values(value)
    for (const field of fields) {
      const input = root.current?.querySelector<HTMLInputElement>(
        `[name="${field}"]`,
      )
      if (input) input.value = String(data[field])
    }
  }, [value])
  const defaults = values(value)
  return (
    <div className={styles.metrics} ref={root}>
      {fields.map((field) => (
        <label key={field}>
          {t(`bitmap.metric.${field}`)}
          <input
            type="number"
            name={field}
            disabled={disabled}
            defaultValue={defaults[field]}
            min={field === 'width' || field === 'height' ? 0 : -32768}
            max={field === 'width' || field === 'height' ? 4096 : 32767}
            onChange={() => {
              dirty.current = true
              noteDocumentEdit(session.target.doc)
            }}
            onBlur={() => {
              void commit().catch(reportError)
            }}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault()
                void commit().catch(reportError)
              }
            }}
          />
        </label>
      ))}
    </div>
  )
}
