import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { call, type DocSummary, type GlyphId } from '../../api'
import { Button } from '../../components/Button'
import { Dialog } from '../../components/Dialog'
import { withSettledDocument } from '../../editor/documentPersistence'
import { errorNotice } from '../../i18n/errors'
import { useSessionStore } from '../../stores/sessionStore'
import { documentGlyphs } from './crossDocumentModel'
import styles from './crossDocument.module.css'

/** Repair missing or deliberately replaced component bases in one undo step. */
export function RelinkComponentsDialog({
  target,
  glyphs,
  onClose,
}: {
  target: DocSummary
  glyphs: GlyphId[]
  onClose: () => void
}) {
  const { t } = useTranslation()
  const [from, setFrom] = useState('')
  const [to, setTo] = useState('')
  const [allGlyphs, setAllGlyphs] = useState(glyphs.length === 0)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const apply = async () => {
    setBusy(true)
    setError(null)
    try {
      await withSettledDocument(target.doc, async () => {
        const inventory = await documentGlyphs(target.doc)
        const replacement = inventory.find((glyph) => glyph.name === to.trim())
        if (!replacement)
          throw new Error(t('crossDocument.unknownGlyph', { name: to.trim() }))
        await call('doc.relink_components', {
          doc: target.doc,
          from: from.trim(),
          to: replacement.gid,
          gids: allGlyphs ? inventory.map((glyph) => glyph.gid) : glyphs,
          master: null,
          expected_generation: null,
        })
      })
      await useSessionStore.getState().refreshSummary(target.doc)
      onClose()
    } catch (reason) {
      const notice = errorNotice(reason, t)
      setError([notice.message, notice.details].filter(Boolean).join(' · '))
    } finally {
      setBusy(false)
    }
  }
  return (
    <Dialog
      title={t('crossDocument.relinkTitle')}
      onCancel={() => {
        if (!busy) onClose()
      }}
      footer={
        <>
          <Button disabled={busy} onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button
            variant="primary"
            disabled={
              busy ||
              !from.trim() ||
              !to.trim() ||
              (!allGlyphs && glyphs.length === 0)
            }
            onClick={() => {
              void apply()
            }}
          >
            {t('crossDocument.relink')}
          </Button>
        </>
      }
    >
      <fieldset className={styles.form} disabled={busy}>
        <p className={styles.help}>{t('crossDocument.relinkHelp')}</p>
        <label className={styles.field}>
          {t('crossDocument.oldBase')}
          <input
            value={from}
            onChange={(event) => setFrom(event.target.value)}
          />
        </label>
        <label className={styles.field}>
          {t('crossDocument.newBase')}
          <input value={to} onChange={(event) => setTo(event.target.value)} />
        </label>
        <label>
          <input
            type="checkbox"
            checked={allGlyphs}
            onChange={(event) => setAllGlyphs(event.target.checked)}
          />{' '}
          {t('crossDocument.allGlyphs')}
        </label>
        {!allGlyphs && (
          <p className={styles.help}>
            {t('crossDocument.targetSelection', { count: glyphs.length })}
          </p>
        )}
      </fieldset>
      {error && (
        <p role="alert" className={styles.error}>
          {error}
        </p>
      )}
    </Dialog>
  )
}
