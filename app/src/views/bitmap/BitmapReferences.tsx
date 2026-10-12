import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { call, type BitmapGlyph } from '../../api'
import { registerDocumentEditParticipant } from '../../editor/documentEditParticipants'
import { noteDocumentEdit } from '../../editor/editSession'
import { reportError } from '../../stores/uiStore'
import { BitmapSession } from './bitmapSession'
import styles from './BitmapView.module.css'

/** Edits translated bitmap components while keeping their original pixels. */
export function BitmapReferences({
  session,
  value,
  disabled,
}: {
  session: BitmapSession
  value: BitmapGlyph
  disabled: boolean
}) {
  const { t } = useTranslation()
  const root = useRef<HTMLFormElement>(null)
  const dirty = useRef(false)
  const [pending, setPending] = useState(false)
  const [base, setBase] = useState('')
  const [dx, setDx] = useState('0')
  const [dy, setDy] = useState('0')
  const blocked = disabled || pending
  const offset = (text: string) => {
    const number = Number(text)
    if (
      !text.trim() ||
      !Number.isInteger(number) ||
      number < -32768 ||
      number > 32767
    )
      throw new Error(t('bitmapReferences.invalidOffset'))
    return number
  }
  const current = () => {
    const state = session.snapshot()
    if (state.busy || state.suspended || !state.value)
      throw new Error(t('bitmapReferences.busy'))
    return state.value
  }
  const commitOffsets = async () => {
    if (!dirty.current) return
    const previous = current()
    const refs = previous.refs.map((reference, index) => ({
      ...reference,
      ...Object.fromEntries(
        (['dx', 'dy'] as const).map((axis) => {
          const input = root.current?.querySelector<HTMLInputElement>(
            `[name="${index}.${axis}"]`,
          )
          return [axis, input ? offset(input.value) : reference[axis]]
        }),
      ),
    }))
    if (JSON.stringify(refs) !== JSON.stringify(previous.refs))
      session.edit({ ...previous, refs })
    dirty.current = false
    await session.flush()
  }
  const latest = useRef(commitOffsets)
  useEffect(() => {
    latest.current = commitOffsets
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
    value.refs.forEach((reference, index) => {
      for (const axis of ['dx', 'dy'] as const) {
        const input = root.current?.querySelector<HTMLInputElement>(
          `[name="${index}.${axis}"]`,
        )
        if (input) input.value = String(reference[axis])
      }
    })
  }, [value])
  const run = (operation: () => Promise<void>) => {
    if (blocked) return
    setPending(true)
    void operation()
      .catch(reportError)
      .finally(() => setPending(false))
  }
  const add = async () => {
    const name = base.trim()
    if (!name || name === session.target.glyph)
      throw new Error(t('bitmapReferences.invalidBase'))
    const reference = { base: name, dx: offset(dx), dy: offset(dy) }
    await commitOffsets()
    const previous = current()
    session.edit({ ...previous, refs: [...previous.refs, reference] })
    await session.flush()
    setBase('')
  }
  const remove = async (index: number) => {
    await commitOffsets()
    const previous = current()
    session.edit({
      ...previous,
      refs: previous.refs.filter((_, at) => at !== index),
    })
    await session.flush()
  }
  const flatten = async () => {
    await commitOffsets()
    await session.flush()
    const previous = current()
    const result = await call('bitmap.glyph', session.target)
    // Another editor action may have occurred while resolving the components.
    if (current() !== previous || session.snapshot().dirty)
      throw new Error(t('bitmapReferences.busy'))
    if (!result.resolved) throw new Error(t('bitmapReferences.missingResolved'))
    session.edit({ ...previous, bitmap: result.resolved.bitmap, refs: [] })
    await session.flush()
  }
  return (
    <details open>
      <summary>{t('bitmapReferences.title')}</summary>
      <p className={styles.hint}>{t('bitmapReferences.hint')}</p>
      <form
        ref={root}
        onSubmit={(event) => {
          event.preventDefault()
          run(commitOffsets)
        }}
      >
        {!value.refs.length && <p>{t('bitmapReferences.empty')}</p>}
        {value.refs.map((reference, index) => (
          <div key={`${index}.${reference.base}`} className={styles.toolbar}>
            <code>{reference.base}</code>
            {(['dx', 'dy'] as const).map((axis) => (
              <label key={axis}>
                {t(`bitmapReferences.${axis}`)}
                <input
                  name={`${index}.${axis}`}
                  type="number"
                  min={-32768}
                  max={32767}
                  required
                  disabled={blocked}
                  defaultValue={reference[axis]}
                  aria-label={t('bitmapReferences.offset', {
                    glyph: reference.base,
                    axis: t(`bitmapReferences.${axis}`),
                  })}
                  onChange={() => {
                    dirty.current = true
                    noteDocumentEdit(session.target.doc)
                  }}
                />
              </label>
            ))}
            <button
              type="button"
              disabled={blocked}
              onClick={() => run(() => remove(index))}
            >
              {t('bitmapReferences.remove', { glyph: reference.base })}
            </button>
          </div>
        ))}
        {!!value.refs.length && (
          <div className={styles.toolbar}>
            <button disabled={blocked}>{t('bitmapReferences.apply')}</button>
            <button
              type="button"
              disabled={blocked}
              onClick={() => run(flatten)}
            >
              {t('bitmapReferences.flatten')}
            </button>
          </div>
        )}
      </form>
      <form
        className={styles.toolbar}
        onSubmit={(event) => {
          event.preventDefault()
          run(add)
        }}
      >
        <label>
          {t('bitmapReferences.base')}
          <input
            required
            value={base}
            disabled={blocked}
            onChange={(event) => setBase(event.target.value)}
          />
        </label>
        <label>
          {t('bitmapReferences.dx')}
          <input
            type="number"
            min={-32768}
            max={32767}
            required
            value={dx}
            disabled={blocked}
            onChange={(event) => setDx(event.target.value)}
          />
        </label>
        <label>
          {t('bitmapReferences.dy')}
          <input
            type="number"
            min={-32768}
            max={32767}
            required
            value={dy}
            disabled={blocked}
            onChange={(event) => setDy(event.target.value)}
          />
        </label>
        <button disabled={blocked || !base.trim()}>
          {t('bitmapReferences.add')}
        </button>
      </form>
    </details>
  )
}
