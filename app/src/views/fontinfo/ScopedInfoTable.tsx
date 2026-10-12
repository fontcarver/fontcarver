import { useEffect, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'

import {
  call,
  type Axis,
  type DocId,
  type FontInfo,
  type Instance,
  type Master,
} from '../../api'

import {
  matchingFields,
  scopedFields,
  type InfoScope,
  type ScopeEntry,
} from './scopedInfo'
import styles from './fontInfo.module.css'

const SCOPES: readonly InfoScope[] = ['family', 'font', 'master', 'export']

/** Per-source values of one scope, marked when they really replace the value above. */
function Entries({ entries }: { entries: readonly ScopeEntry[] }) {
  const { t } = useTranslation()
  return (
    <ul className={styles.scopeEntries}>
      {entries.map((entry) => (
        <li key={entry.name} data-overridden={entry.overridden}>
          <span>{entry.name}</span> <strong>{entry.value}</strong>
          {entry.derived && (
            <small data-derived="">{t('fontInfo.scoped.derived')}</small>
          )}
          {entry.overridden && <small>{t('fontInfo.scoped.overridden')}</small>}
        </li>
      ))}
    </ul>
  )
}

/**
 * The scope breadcrumb for every info field.
 *
 * A value inherits down Family → Font → Master → Export, and the cell that
 * supplies what a compiled export actually receives is marked. A value an
 * export merely repeats is not an override; a class the compiler derives from
 * the instance location is shown as derived.
 */
export function ScopedInfoTable({
  heading,
  doc,
  info,
  masters,
  instances,
  query,
}: {
  heading?: string
  doc: DocId
  info: FontInfo
  masters: readonly Master[]
  instances: readonly Instance[]
  query: string
}) {
  const { t } = useTranslation()
  // The registered-axis mapping is what turns an instance location into the
  // weight and width classes the compiler derives, and `doc.font_info` does
  // not carry it.
  const [axes, setAxes] = useState<Axis[]>([])
  useEffect(() => {
    let current = true
    void call('variation.state', { doc })
      .then((state) => {
        if (current) setAxes(state.axes)
      })
      .catch(() => undefined)
    return () => {
      current = false
    }
  }, [doc])
  const fields = useMemo(
    () => scopedFields(info, masters, instances, axes),
    [info, masters, instances, axes],
  )
  const shown = useMemo(
    () =>
      matchingFields(fields, (id) => t(`fontInfo.scoped.field.${id}`), query),
    [fields, query, t],
  )
  if (shown.length === 0)
    return heading ? null : <p>{t('fontInfo.noMatches')}</p>
  return (
    <>
      {heading && <h3>{heading}</h3>}
      <p className={styles.scopeHint}>{t('fontInfo.scoped.hint')}</p>
      <table className={styles.table}>
        <caption className={styles.visuallyHidden}>
          {t('fontInfo.scoped.caption')}
        </caption>
        <thead>
          <tr>
            <th scope="col">{t('fontInfo.scoped.fieldColumn')}</th>
            {SCOPES.map((scope) => (
              <th key={scope} scope="col">
                {t(`fontInfo.scoped.scope.${scope}`)}
              </th>
            ))}
            <th scope="col">{t('fontInfo.scoped.effective')}</th>
          </tr>
        </thead>
        <tbody>
          {shown.map((field) => (
            <tr key={field.id} data-field={field.id}>
              <th scope="row">{t(`fontInfo.scoped.field.${field.id}`)}</th>
              {SCOPES.map((scope) => (
                <td
                  key={scope}
                  data-scope={scope}
                  data-effective={field.effective === scope}
                >
                  {scope === 'family' && (field.family ?? '—')}
                  {scope === 'font' && (field.font ?? '—')}
                  {scope === 'master' &&
                    (field.master ? <Entries entries={field.master} /> : '—')}
                  {scope === 'export' &&
                    (field.export ? <Entries entries={field.export} /> : '—')}
                </td>
              ))}
              <td data-effective-scope={field.effective}>
                {t(`fontInfo.scoped.scope.${field.effective}`)}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </>
  )
}
