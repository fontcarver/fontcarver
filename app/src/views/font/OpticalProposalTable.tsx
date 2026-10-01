import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { OpticalProposalEntry } from '../../api'
import { Button } from '../../components/Button'

import styles from './opticalSpacing.module.css'

/** Rows added to the visible window by one "show more". */
const PAGE = 200

/** Stable identity of one proposal, which is what an acceptance sends back. */
export function proposalKey(entry: OpticalProposalEntry): string {
  return `${entry.glyph}\u0000${entry.right ?? ''}\u0000${entry.master}`
}

/**
 * The review table: every proposed change with its own accept box.
 *
 * Applying sends exactly the accepted rows, so a reviewer can take the pairs
 * they agree with and leave the rest. Large proposals are shown a window at a
 * time; select all and select none always act on every row, not just the window.
 */
export function OpticalProposalTable({
  proposals,
  rejected,
  onRejected,
}: {
  proposals: OpticalProposalEntry[]
  rejected: ReadonlySet<string>
  onRejected: (next: Set<string>) => void
}) {
  const { t } = useTranslation()
  const [limit, setLimit] = useState(PAGE)
  const shown = proposals.slice(0, limit)
  const toggle = (key: string, accept: boolean) => {
    const next = new Set(rejected)
    if (accept) next.delete(key)
    else next.add(key)
    onRejected(next)
  }
  return (
    <div className={styles.proposals}>
      <div className={styles.proposalActions}>
        <Button onClick={() => onRejected(new Set())}>
          {t('opticalSpacing.acceptAll')}
        </Button>
        <Button onClick={() => onRejected(new Set(proposals.map(proposalKey)))}>
          {t('opticalSpacing.rejectAll')}
        </Button>
        <span>
          {t('opticalSpacing.acceptedCount', {
            count: proposals.length - rejected.size,
            total: proposals.length,
          })}
        </span>
      </div>
      <table className={styles.proposalTable}>
        <caption>{t('opticalSpacing.reviewTable')}</caption>
        <thead>
          <tr>
            <th scope="col">{t('opticalSpacing.accept')}</th>
            <th scope="col">{t('opticalSpacing.subject')}</th>
            <th scope="col">{t('opticalSpacing.master')}</th>
            <th scope="col">{t('opticalSpacing.before')}</th>
            <th scope="col">{t('opticalSpacing.after')}</th>
          </tr>
        </thead>
        <tbody>
          {shown.map((entry) => {
            const key = proposalKey(entry)
            // A row describes itself: a pair has a right glyph, a layer has not.
            const pair = entry.right !== null
            const subject = pair
              ? t('opticalSpacing.pairLabel', {
                  left: entry.glyph,
                  right: entry.right,
                })
              : entry.glyph
            const values = (triple: [number, number, number]) =>
              pair
                ? String(Math.round(triple[0] * 100) / 100)
                : triple
                    .map((value) => Math.round(value * 100) / 100)
                    .join(' / ')
            return (
              <tr key={key}>
                <td>
                  <input
                    type="checkbox"
                    checked={!rejected.has(key)}
                    aria-label={t('opticalSpacing.acceptRow', { subject })}
                    onChange={(event) => toggle(key, event.target.checked)}
                  />
                </td>
                <th scope="row">{subject}</th>
                <td>{entry.master_name}</td>
                <td>{values(entry.before)}</td>
                <td>{values(entry.after)}</td>
              </tr>
            )
          })}
        </tbody>
      </table>
      {proposals.length > shown.length && (
        <div className={styles.proposalActions}>
          <Button onClick={() => setLimit(limit + PAGE)}>
            {t('opticalSpacing.showMore')}
          </Button>
          <span>
            {t('opticalSpacing.windowed', {
              shown: shown.length,
              total: proposals.length,
            })}
          </span>
        </div>
      )}
    </div>
  )
}
