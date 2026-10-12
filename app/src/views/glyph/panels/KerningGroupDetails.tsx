import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { call, type KerningGroupSummary } from '../../../api'

import { KerningGroupPager } from './KerningGroupPager'
import type { KerningGroupSideEditorProps } from './KerningGroupSideEditor'
import { canonicalGroupName } from './kerningGroupEdits'
import styles from './kerningGroups.module.css'
import shared from './panels.module.css'
import { useKerningGroupQuery } from './useKerningGroupQuery'

/** Group-wide impact, bounded membership pages, rename and explicit removal. */
export function KerningGroupDetails({
  doc,
  glyph,
  side,
  membership,
  status,
  group,
  run,
  refresh,
}: KerningGroupSideEditorProps & { group: KerningGroupSummary }) {
  const { t } = useTranslation()
  const [offset, setOffset] = useState(0)
  const [name, setName] = useState(group.name)
  const [confirmDelete, setConfirmDelete] = useState(false)
  const generation = status.generation
  const identity = JSON.stringify([doc, side, group.name, generation, offset])
  const answer = useKerningGroupQuery(identity, () =>
    call('kerning.members', {
      doc,
      side,
      name: group.name,
      generation,
      offset,
      limit: 25,
    }),
  )
  const members = answer?.value
  const rename = () => {
    if (!name.trim() || canonicalGroupName(side, name) === group.name) return
    run(
      {
        RenameKerningGroup: {
          side,
          from: group.name,
          to: canonicalGroupName(side, name),
        },
      },
      t('kerningGroups.undo.rename'),
    )
  }
  const remove = () =>
    run(
      {
        RemoveKerningGroup: {
          side,
          name: group.name,
          remove_pairs: group.pair_count > 0,
        },
      },
      t('kerningGroups.undo.delete'),
    )

  return (
    <div className={styles.detail}>
      <p className={shared.hint}>
        {t('kerningGroups.pairs', { count: group.pair_count })}
      </p>
      {group.diagnostic && (
        <p className={shared.hint} role="alert" title={group.diagnostic}>
          {t('kerningGroups.invalidMembership')}
        </p>
      )}
      <div
        role="region"
        aria-label={t('kerningGroups.memberList', { name: group.name })}
      >
        <ul className={styles.members}>
          {(members?.items ?? group.sample_members).map((member, index) => (
            <li key={`${member}:${index}`}>{member}</li>
          ))}
        </ul>
        {members && (
          <KerningGroupPager
            offset={offset}
            total={members.total}
            onPage={setOffset}
          />
        )}
        {answer?.failed && (
          <button
            type="button"
            className={shared.actionButton}
            onClick={refresh}
          >
            {t('kerningGroups.refresh')}
          </button>
        )}
      </div>
      <button
        type="button"
        className={shared.actionButton}
        disabled={membership === group.name || Boolean(group.diagnostic)}
        onClick={() =>
          run(
            { AssignKerningGroup: { side, glyph, group: group.name } },
            t('kerningGroups.undo.assign'),
          )
        }
      >
        {t('kerningGroups.assign', { glyph })}
      </button>
      <input
        className={shared.textInput}
        aria-label={t('kerningGroups.renameName', { name: group.name })}
        value={name}
        onChange={(event) => setName(event.currentTarget.value)}
        onKeyDown={(event) => {
          if (event.key === 'Escape') setName(group.name)
          if (event.key === 'Enter') {
            event.preventDefault()
            rename()
          }
        }}
      />
      <button
        type="button"
        className={shared.actionButton}
        disabled={
          !name.trim() ||
          canonicalGroupName(side, name) === group.name ||
          Boolean(group.diagnostic)
        }
        onClick={rename}
      >
        {t('kerningGroups.rename')}
      </button>
      {!confirmDelete && (
        <button
          type="button"
          className={shared.actionButton}
          disabled={Boolean(group.diagnostic)}
          onClick={() => setConfirmDelete(true)}
        >
          {t('kerningGroups.delete')}
        </button>
      )}
      {confirmDelete && (
        <div className={styles.confirm}>
          <p className={shared.hint}>
            {t('kerningGroups.confirmDelete', {
              name: group.name,
              members: group.member_count,
            })}
          </p>
          {group.pair_count > 0 && (
            <p className={shared.hint}>
              {t('kerningGroups.confirmPairs', { count: group.pair_count })}
            </p>
          )}
          <button
            type="button"
            className={shared.actionButton}
            onClick={remove}
          >
            {t(
              group.pair_count > 0
                ? 'kerningGroups.deleteGroupAndPairs'
                : 'kerningGroups.deleteGroup',
            )}
          </button>
          <button
            type="button"
            className={shared.actionButton}
            onClick={() => setConfirmDelete(false)}
          >
            {t('kerningGroups.cancel')}
          </button>
        </div>
      )}
    </div>
  )
}
