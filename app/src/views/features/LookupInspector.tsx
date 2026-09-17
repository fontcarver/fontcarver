import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { call, type DocId, type MasterId } from '../../api'
import { selectDocView, useDocumentStore } from '../../stores/documentStore'
import { useSessionStore } from '../../stores/sessionStore'

import { LayoutConversion } from './LayoutConversion'
import { LookupRule } from './LookupRule'
import styles from './LookupInspector.module.css'
import { useLayoutQuery } from './useLayoutQuery'

/** Context for the font's read-only original layout and explicit conversion. */
export interface LookupInspectorProps {
  doc: DocId
  generation: number
  master?: MasterId | null
  onChanged: () => void
  beforeChange?: () => Promise<void>
}

const LOOKUPS_PER_PAGE = 25
const RULES_PER_PAGE = 10

/** Inspects preserved lookup order and rules without changing layout authority. */
export function LookupInspector({
  doc,
  generation,
  master: suppliedMaster,
  onChanged,
  beforeChange,
}: LookupInspectorProps) {
  const { t } = useTranslation()
  const selectedMaster = useDocumentStore(selectDocView(doc)).master
  const defaultMaster = useSessionStore(
    (state) =>
      state.docs.find((entry) => entry.doc === doc)?.masters[0]?.id ?? null,
  )
  const master = suppliedMaster ?? selectedMaster ?? defaultMaster
  const identity = JSON.stringify([doc, generation, master])
  const [context, setContext] = useState(identity)
  const [lookupOffset, setLookupOffset] = useState(0)
  const [ruleOffset, setRuleOffset] = useState(0)
  const [selection, setSelection] = useState('')
  const [refresh, setRefresh] = useState(0)
  if (context !== identity) {
    setContext(identity)
    setLookupOffset(0)
    setRuleOffset(0)
    setSelection('')
  }
  const inspectionAnswer = useLayoutQuery(
    `${identity}:${lookupOffset}:${refresh}`,
    () =>
      call('layout.inspect', {
        doc,
        generation,
        offset: lookupOffset,
        limit: LOOKUPS_PER_PAGE,
      }),
  )
  const inspection = inspectionAnswer?.value
  const selected =
    inspection?.items.find(
      (item) => `${item.table}:${item.index}` === selection,
    ) ?? inspection?.items[0]
  const rulesAnswer = useLayoutQuery(
    JSON.stringify([
      identity,
      lookupOffset,
      selected?.table,
      selected?.index,
      ruleOffset,
      refresh,
      inspection?.compile_revision,
      inspection?.draft_revision,
    ]),
    async () => {
      if (!selected || !inspection) return null
      const result = await call('layout.rules', {
        doc,
        table: selected.table,
        lookup: selected.index,
        generation: inspection.generation,
        offset: ruleOffset,
        limit: RULES_PER_PAGE,
      })
      if (
        result.generation !== inspection.generation ||
        result.compile_revision !== inspection.compile_revision ||
        result.draft_revision !== inspection.draft_revision ||
        result.fingerprint !== inspection.fingerprint
      )
        throw new Error('Stale layout rules')
      return result
    },
  )
  const rules = rulesAnswer?.value
  const retry = () => setRefresh((value) => value + 1)
  const changeLookupPage = (offset: number) => {
    setLookupOffset(offset)
    setRuleOffset(0)
    setSelection('')
  }

  if (!inspection)
    return (
      <section className={styles.panel} aria-label={t('layoutInspector.title')}>
        <p
          role={inspectionAnswer?.failed ? 'alert' : 'status'}
          className={styles.hint}
        >
          {t(
            inspectionAnswer?.failed
              ? 'layoutInspector.loadFailed'
              : 'layoutInspector.loading',
          )}
        </p>
        {inspectionAnswer?.failed && (
          <button type="button" className={styles.button} onClick={retry}>
            {t('layoutInspector.retry')}
          </button>
        )}
      </section>
    )

  return (
    <section className={styles.panel} aria-label={t('layoutInspector.title')}>
      <h3>{t('layoutInspector.title')}</h3>
      <p className={styles.hint}>{t('layoutInspector.readOnly')}</p>
      <p className={styles.hint}>
        {t(
          inspection.authority === 'ImportedTables'
            ? 'layoutInspector.originalActive'
            : 'layoutInspector.sourceActive',
        )}
      </p>
      <div className={styles.tables} aria-label={t('layoutInspector.tables')}>
        {inspection.tables.map((table) => (
          <span key={table.tag}>
            {t('layoutInspector.table', {
              tag: table.tag,
              count: table.lookup_count,
            })}
          </span>
        ))}
      </div>
      {inspection.tables.length > 0 && (
        <LayoutConversion
          inspection={inspection}
          master={master}
          onChanged={onChanged}
          beforeChange={beforeChange}
        />
      )}
      {inspection.blockers.length > 0 && (
        <ul className={styles.list} aria-label={t('layoutInspector.blockers')}>
          {inspection.blockers.map((blocker, index) => (
            <li key={index} className={styles.hint}>
              <code>
                {blocker.table}
                {blocker.lookup === null ? '' : ` #${blocker.lookup}`}
              </code>
              {' · '}
              {blocker.reason}
            </li>
          ))}
        </ul>
      )}
      {inspection.blockers_truncated && (
        <p className={styles.hint}>{t('layoutInspector.moreBlockers')}</p>
      )}
      {inspection.total === 0 && (
        <p className={styles.hint}>{t('layoutInspector.empty')}</p>
      )}
      {selected && (
        <>
          <div className={styles.controls}>
            <label htmlFor={`lookup-${doc}`}>
              {t('layoutInspector.lookup')}
            </label>
            <select
              id={`lookup-${doc}`}
              className={styles.select}
              value={`${selected.table}:${selected.index}`}
              onChange={(event) => {
                setSelection(event.currentTarget.value)
                setRuleOffset(0)
              }}
            >
              {inspection.items.map((item) => (
                <option
                  key={`${item.table}:${item.index}`}
                  value={`${item.table}:${item.index}`}
                >
                  {t('layoutInspector.lookupOption', {
                    table: item.table,
                    index: item.index,
                    kind: item.kind,
                  })}
                </option>
              ))}
            </select>
          </div>
          <p className={styles.metadata}>
            {t('layoutInspector.flags', {
              flags: selected.flags,
              count: selected.rule_count,
            })}
          </p>
          {selected.referenced_lookups.length > 0 && (
            <p className={styles.metadata}>
              {t('layoutInspector.references', {
                lookups: selected.referenced_lookups.join(', '),
              })}
              {selected.references_truncated ? '…' : ''}
            </p>
          )}
          {(lookupOffset > 0 || inspection.next_offset !== null) && (
            <div className={styles.pager}>
              <button
                type="button"
                className={styles.button}
                disabled={lookupOffset === 0}
                onClick={() =>
                  changeLookupPage(Math.max(0, lookupOffset - LOOKUPS_PER_PAGE))
                }
              >
                {t('layoutInspector.previousLookups')}
              </button>
              <span className={styles.hint}>
                {t('layoutInspector.page', {
                  first: lookupOffset + 1,
                  last: lookupOffset + inspection.items.length,
                  total: inspection.total,
                })}
              </span>
              <button
                type="button"
                className={styles.button}
                disabled={inspection.next_offset === null}
                onClick={() => {
                  if (inspection.next_offset !== null)
                    changeLookupPage(inspection.next_offset)
                }}
              >
                {t('layoutInspector.nextLookups')}
              </button>
            </div>
          )}
          {!rules && (
            <p
              className={styles.hint}
              role={rulesAnswer?.failed ? 'alert' : 'status'}
            >
              {t(
                rulesAnswer?.failed
                  ? 'layoutInspector.rulesFailed'
                  : 'layoutInspector.loading',
              )}
            </p>
          )}
          {rulesAnswer?.failed && (
            <button type="button" className={styles.button} onClick={retry}>
              {t('layoutInspector.retry')}
            </button>
          )}
          {rules && (
            <>
              <ol
                className={styles.list}
                aria-label={t('layoutInspector.rules')}
              >
                {rules.items.map((rule, index) => (
                  <LookupRule
                    key={`${identity}:${selected.table}:${selected.index}:${ruleOffset + index}`}
                    doc={doc}
                    master={master}
                    generation={rules.generation}
                    table={selected.table}
                    lookup={selected.index}
                    index={ruleOffset + index}
                    rule={rule}
                  />
                ))}
              </ol>
              {rules.total === 0 && (
                <p className={styles.hint}>{t('layoutInspector.noRules')}</p>
              )}
              {(ruleOffset > 0 || rules.next_offset !== null) && (
                <div className={styles.pager}>
                  <button
                    type="button"
                    className={styles.button}
                    disabled={ruleOffset === 0}
                    onClick={() =>
                      setRuleOffset(Math.max(0, ruleOffset - RULES_PER_PAGE))
                    }
                  >
                    {t('layoutInspector.previousRules')}
                  </button>
                  <span className={styles.hint}>
                    {t('layoutInspector.page', {
                      first: ruleOffset + 1,
                      last: ruleOffset + rules.items.length,
                      total: rules.total,
                    })}
                  </span>
                  <button
                    type="button"
                    className={styles.button}
                    disabled={rules.next_offset === null}
                    onClick={() => {
                      if (rules.next_offset !== null)
                        setRuleOffset(rules.next_offset)
                    }}
                  >
                    {t('layoutInspector.nextRules')}
                  </button>
                </div>
              )}
            </>
          )}
        </>
      )}
    </section>
  )
}
