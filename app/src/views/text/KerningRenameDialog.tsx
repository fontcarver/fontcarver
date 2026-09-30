import { useEffect, useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  call,
  type KerningGroupSide,
  type KerningGroupsResult,
} from '../../api'
import { Dialog } from '../../components/Dialog'
import { canonicalGroupName } from '../glyph/panels/kerningGroupEdits'
import styles from './KerningTable.module.css'

/** How widely a kerning-group rename applies. */
export type KerningRenameScope = 'font' | 'record'

/** Largest page of existing group names offered as completions. */
const GROUP_PAGE = 200

/**
 * Asks how widely a kerning-group rename should reach.
 *
 * The two answers are genuinely different edits, so the dialog asks rather than
 * assuming. *Font-wide* renames the group itself: glyph membership follows, and
 * every reference in both directions and every master is rewritten. *This
 * record only* leaves the group alone and retargets the single row at the new
 * group, which is how one pair is moved to a different class without disturbing
 * the rest of the table.
 *
 * Retargeting needs a readable value to carry across and a group to carry it
 * to, so that scope is offered only for a record the table could read, and the
 * name field completes from the groups that already exist on this side. A name
 * that is not one of them is still allowed — the kernel creates it as an empty
 * group — but the field says so rather than letting a typo look deliberate.
 */
export function KerningRenameDialog({
  name,
  side,
  doc,
  busy,
  canRetarget,
  onCancel,
  onRename,
}: {
  name: string
  side: KerningGroupSide
  /** Document whose existing groups are offered as completions. */
  doc: number
  busy: boolean
  /** Whether this record can be retargeted: it needs a readable value. */
  canRetarget: boolean
  onCancel: () => void
  onRename: (name: string, scope: KerningRenameScope) => void
}) {
  const { t } = useTranslation()
  const [draft, setDraft] = useState(name)
  const [scope, setScope] = useState<KerningRenameScope>('font')
  const [groups, setGroups] = useState<readonly string[]>([])
  const listId = useId()
  useEffect(() => {
    let current = true
    void call('kerning.groups', {
      doc,
      side,
      generation: null,
      query: null,
      offset: 0,
      limit: GROUP_PAGE,
    }).then(
      (result: KerningGroupsResult) => {
        if (current) setGroups(result.items.map((group) => group.name))
      },
      () => {
        // Completions are a convenience; typing a name still works without them.
        if (current) setGroups([])
      },
    )
    return () => {
      current = false
    }
  }, [doc, side])

  const target = canonicalGroupName(side, draft)
  const valid = draft.trim().length > 0 && target !== name
  const known = groups.includes(target)
  const confirm = () => {
    if (valid && !busy) onRename(target, scope)
  }
  return (
    <Dialog
      title={t('kerningTable.renameTitle')}
      onCancel={onCancel}
      onConfirm={confirm}
      footer={
        <>
          <button type="button" disabled={busy} onClick={onCancel}>
            {t('common.cancel')}
          </button>
          <button type="button" disabled={busy || !valid} onClick={confirm}>
            {t('kerningTable.renameConfirm')}
          </button>
        </>
      }
    >
      <label className={styles.renameName}>
        {t('kerningTable.renameName')}
        <input
          value={draft}
          list={listId}
          onChange={(event) => setDraft(event.currentTarget.value)}
          disabled={busy}
        />
      </label>
      <datalist id={listId}>
        {groups.map((group) => (
          <option key={group} value={group} />
        ))}
      </datalist>
      <fieldset className={styles.renameScope}>
        <legend>{t('kerningTable.renameScopeLabel')}</legend>
        {(['font', 'record'] as const).map((option) => (
          <label key={option}>
            <input
              type="radio"
              name="kerning-rename-scope"
              value={option}
              checked={scope === option}
              disabled={busy || (option === 'record' && !canRetarget)}
              onChange={() => setScope(option)}
            />
            {t(`kerningTable.renameScopes.${option}`)}
          </label>
        ))}
      </fieldset>
      <p>
        {t(
          scope === 'font'
            ? 'kerningTable.renameScope'
            : 'kerningTable.renameRecordScope',
          { name },
        )}
      </p>
      {!canRetarget && (
        <p className={styles.hint}>{t('kerningTable.retargetUnavailable')}</p>
      )}
      {scope === 'record' && valid && !known && (
        <p className={styles.hint} role="status">
          {t('kerningTable.renameNewGroup', { name: target })}
        </p>
      )}
    </Dialog>
  )
}
