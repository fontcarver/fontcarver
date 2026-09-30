import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { SpacingRow } from '../../api'
import { metricsKeyError } from '../glyph/panels/metricsKeyCommands'
import { cellText, type SpacingField } from './spacingMatrix'
import styles from './SpacingMatrix.module.css'

const FIELDS: readonly SpacingField[] = ['left', 'right', 'width']

function round(value: number): string {
  return String(Math.round(value * 10) / 10)
}

/** Whether typed text is a formula rather than a plain number. */
function isFormula(text: string): boolean {
  const trimmed = text.trim()
  return trimmed !== '' && !/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)$/u.test(trimmed)
}

/**
 * One matrix cell: a number or a metrics formula, in the same box.
 *
 * The field follows the model between edits, so an undo or an edit made
 * elsewhere is visible immediately, but keeps a rejected draft on screen so it
 * can be corrected. A formula is validated before it is sent.
 */
function SpacingCell({
  label,
  text,
  disabled,
  onCommit,
}: {
  label: string
  text: string
  disabled: boolean
  onCommit: (raw: string) => void
}) {
  const { t } = useTranslation()
  const [draft, setDraft] = useState(text)
  const [shown, setShown] = useState(text)
  const cancelBlur = useRef(false)
  if (text !== shown) {
    setShown(text)
    setDraft(text)
  }
  const problem = isFormula(draft) ? metricsKeyError(draft) : null
  const commit = (raw: string) => {
    if (disabled || (isFormula(raw) && metricsKeyError(raw))) return
    onCommit(raw)
  }
  const message = problem ? t(`panel.metrics.keyError.${problem}`) : null
  return (
    <td>
      <input
        className={styles.cell}
        type="text"
        spellCheck={false}
        disabled={disabled}
        value={draft}
        aria-label={label}
        aria-invalid={problem ? true : undefined}
        title={message ?? undefined}
        onChange={(event) => setDraft(event.currentTarget.value)}
        onFocus={(event) => event.currentTarget.select()}
        onBlur={(event) => {
          if (cancelBlur.current) {
            cancelBlur.current = false
            return
          }
          commit(event.currentTarget.value)
        }}
        onKeyDown={(event) => {
          if (event.key === 'Enter') {
            event.preventDefault()
            commit(event.currentTarget.value)
            return
          }
          if (event.key !== 'Escape') return
          cancelBlur.current = true
          setDraft(text)
          event.currentTarget.blur()
        }}
      />
      {/* The refusal is spoken, not only hovered: a dropped commit the user
          cannot see is indistinguishable from a silent failure. */}
      {message && (
        <span className={styles.problem} role="alert">
          {message}
        </span>
      )}
    </td>
  )
}

/** One glyph's row of the spacing matrix. */
export function SpacingMatrixRow({
  row,
  beam,
  disabled,
  canUpdate,
  onCommit,
  onUpdate,
}: {
  row: SpacingRow
  beam: boolean
  disabled: boolean
  /** Whether a master is selected, which "update this master" requires. */
  canUpdate: boolean
  onCommit: (row: SpacingRow, field: SpacingField, raw: string) => void
  onUpdate: (row: SpacingRow) => void
}) {
  const { t } = useTranslation()
  const readOnly = disabled || row.missing || row.locked || row.layer === null
  return (
    <tr data-glyph={row.glyph}>
      <th scope="row">{row.glyph}</th>
      {FIELDS.map((field) => (
        <SpacingCell
          key={field}
          label={t('spacingMatrix.cell', {
            glyph: row.glyph,
            metric: t(`spacingMatrix.${field}`),
          })}
          text={cellText(row, field)}
          disabled={readOnly}
          onCommit={(raw) => onCommit(row, field, raw)}
        />
      ))}
      {beam && (
        <>
          <td className={styles.measure}>
            {row.beam_left === null ? '—' : round(row.beam_left)}
          </td>
          <td className={styles.measure}>
            {row.beam_right === null ? '—' : round(row.beam_right)}
          </td>
        </>
      )}
      <td>
        {row.missing ? (
          <span className={styles.problem}>{t('spacingMatrix.missing')}</span>
        ) : row.layer === null ? (
          <span className={styles.problem}>{t('spacingMatrix.noLayer')}</span>
        ) : row.key_error !== null ? (
          <span className={styles.problem} title={row.key_error}>
            {t('spacingMatrix.keyError')}
          </span>
        ) : row.out_of_sync ? (
          <button
            type="button"
            className={styles.update}
            disabled={readOnly || !canUpdate}
            title={t(
              canUpdate
                ? 'spacingMatrix.outOfSyncHint'
                : 'spacingMatrix.noMaster',
            )}
            onClick={() => onUpdate(row)}
          >
            {t('spacingMatrix.update')}
          </button>
        ) : (
          <span className={styles.inSync}>{t('spacingMatrix.inSync')}</span>
        )}
        {row.locked && (
          <span className={styles.problem}>{t('spacingMatrix.locked')}</span>
        )}
      </td>
    </tr>
  )
}
