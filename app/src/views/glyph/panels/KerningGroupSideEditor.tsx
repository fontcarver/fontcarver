import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import {
  call,
  type Command,
  type DocId,
  type KerningGroupSide,
  type KerningStatus,
} from '../../../api'

import { KerningGroupDetails } from './KerningGroupDetails'
import { canonicalGroupName } from './kerningGroupEdits'
import { KerningGroupPager } from './KerningGroupPager'
import styles from './kerningGroups.module.css'
import shared from './panels.module.css'
import { useKerningGroupQuery } from './useKerningGroupQuery'

export interface KerningGroupSideEditorProps {
  doc: DocId
  glyph: string
  side: KerningGroupSide
  membership: string | null
  status: KerningStatus
  busy: boolean
  run: (command: Command, label: string) => void
  refresh: () => void
}

/** One logical pair side, with a bounded searchable group picker. */
export function KerningGroupSideEditor(props: KerningGroupSideEditorProps) {
  const { doc, glyph, side, membership, status, busy, run, refresh } = props
  const { t } = useTranslation()
  const [open, setOpen] = useState(false)
  const [query, setQuery] = useState('')
  const [offset, setOffset] = useState(0)
  const [selected, setSelected] = useState<string | null>(null)
  const [newName, setNewName] = useState('')
  const generation = status.generation
  const identity = JSON.stringify([doc, side, generation, query, offset, open])
  const answer = useKerningGroupQuery(identity, async () =>
    open
      ? call('kerning.groups', {
          doc,
          side,
          generation,
          query: query || null,
          offset,
          limit: 25,
        })
      : null,
  )
  const page = answer?.value
  const chosen = page?.items.find((entry) => entry.name === selected)
  const label = t(`kerningGroups.${side}`)
  const prefix = side === 'side1' ? 'public.kern1.' : 'public.kern2.'
  const assign = (group: string | null) =>
    run(
      { AssignKerningGroup: { side, glyph, group } },
      t('kerningGroups.undo.assign'),
    )
  const create = () => {
    if (!newName.trim()) return
    const name = canonicalGroupName(side, newName)
    run(
      {
        Batch: [
          { CreateKerningGroup: { side, name, members: [] } },
          { AssignKerningGroup: { side, glyph, group: name } },
        ],
      },
      t('kerningGroups.undo.create'),
    )
  }

  return (
    <fieldset className={styles.side} disabled={busy}>
      <legend>{label}</legend>
      <span
        className={styles.name}
        aria-label={t('kerningGroups.membership', { side: label })}
      >
        {membership ?? t('kerningGroups.none')}
      </span>
      {status.editable && (
        <>
          <button
            type="button"
            className={shared.actionButton}
            aria-expanded={open}
            onClick={() => setOpen(!open)}
          >
            {t('kerningGroups.manage', { side: label })}
          </button>
          {open && (
            <div className={styles.manager}>
              {membership && (
                <button
                  type="button"
                  className={shared.actionButton}
                  onClick={() => assign(null)}
                >
                  {t('kerningGroups.unassign', { glyph })}
                </button>
              )}
              <input
                type="search"
                className={shared.textInput}
                aria-label={t('kerningGroups.search', { side: label })}
                placeholder={t('kerningGroups.search', { side: label })}
                value={query}
                onChange={(event) => {
                  setQuery(event.currentTarget.value)
                  setOffset(0)
                  setSelected(null)
                }}
              />
              {!page && (
                <p
                  className={shared.hint}
                  role={answer?.failed ? 'alert' : 'status'}
                >
                  {t(
                    answer?.failed
                      ? 'kerningGroups.loadFailed'
                      : 'kerningGroups.loading',
                  )}
                </p>
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
              {page && (
                <>
                  {page.items.length === 0 && (
                    <p className={shared.hint}>{t('kerningGroups.empty')}</p>
                  )}
                  <div className={shared.pickerResults}>
                    {page.items.map((entry) => (
                      <button
                        key={entry.name}
                        type="button"
                        className={styles.group}
                        aria-label={t('kerningGroups.select', {
                          name: entry.name,
                        })}
                        aria-pressed={selected === entry.name}
                        onClick={() =>
                          setSelected(
                            selected === entry.name ? null : entry.name,
                          )
                        }
                      >
                        <span className={styles.name}>{entry.name}</span>
                        <span className={shared.hint}>
                          {t('kerningGroups.members', {
                            count: entry.member_count,
                          })}
                        </span>
                      </button>
                    ))}
                  </div>
                  <KerningGroupPager
                    offset={offset}
                    total={page.total}
                    onPage={(next) => {
                      setOffset(next)
                      setSelected(null)
                    }}
                  />
                </>
              )}
              {chosen && (
                <KerningGroupDetails
                  key={chosen.name}
                  {...props}
                  group={chosen}
                />
              )}
              <div className={styles.detail}>
                <input
                  className={shared.textInput}
                  aria-label={t('kerningGroups.newName', { side: label })}
                  value={newName}
                  onChange={(event) => setNewName(event.currentTarget.value)}
                  onKeyDown={(event) => {
                    if (event.key === 'Escape') setNewName('')
                    if (event.key === 'Enter') {
                      event.preventDefault()
                      create()
                    }
                  }}
                />
                <p className={shared.hint}>
                  {t('kerningGroups.nameHint', { prefix })}
                </p>
                <button
                  type="button"
                  className={shared.actionButton}
                  disabled={!newName.trim()}
                  onClick={create}
                >
                  {t('kerningGroups.create')}
                </button>
              </div>
            </div>
          )}
        </>
      )}
    </fieldset>
  )
}
