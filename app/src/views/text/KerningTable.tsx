import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type {
  Command,
  DocId,
  KernPair,
  KerningGroupSide,
  KerningMaintenanceOperation,
  KernSide,
} from '../../api'
import { documentEditingSuspended } from '../../editor/documentEditParticipants'
import { reportError } from '../../shell/commands'
import { commitKerningGroup } from '../glyph/panels/kerningGroupEdits'
import { kerningSideName } from './kerningModel'
import { KerningRenameDialog } from './KerningRenameDialog'
import {
  type KerningTableSort,
  visibleKerningRecords,
} from './kerningTableData'
import { useKerningTable } from './useKerningTable'
import styles from './KerningTable.module.css'

const PAGE_SIZE = 50

/** Stored pair management, including group rules and horizontal/vertical tables. */
export function KerningTable({
  doc,
  master,
  generation,
  vertical,
  disabled = false,
}: {
  doc: DocId
  master: string | null
  generation: number
  vertical: boolean
  disabled?: boolean
}) {
  const { t } = useTranslation()
  const [open, setOpen] = useState(false)
  const [revision, setRevision] = useState(0)
  const [query, setQuery] = useState('')
  const [sort, setSort] = useState<KerningTableSort>('left')
  const [descending, setDescending] = useState(false)
  const [page, setPage] = useState(0)
  const [busy, setBusy] = useState(false)
  const [failed, setFailed] = useState(false)
  const [replace, setReplace] = useState(false)
  const [clipboard, setClipboard] = useState<{
    doc: DocId
    master: string
    vertical: boolean
    records: KernPair[]
  } | null>(null)
  const [rename, setRename] = useState<{
    name: string
    side: KerningGroupSide
  } | null>(null)
  const answer = useKerningTable(
    doc,
    master,
    vertical,
    generation,
    open,
    revision,
  )
  const data = answer?.data
  const records = useMemo(
    () => visibleKerningRecords(data?.items ?? [], query, sort, descending),
    [data, query, sort, descending],
  )
  const maxPage = Math.max(0, Math.ceil(records.length / PAGE_SIZE) - 1)
  const activePage = Math.min(page, maxPage)
  const editable = Boolean(
    data?.status.editable && master && !busy && !disabled,
  )
  const copied =
    clipboard?.doc === doc && clipboard.vertical === vertical ? clipboard : null
  const canCopy =
    data &&
    !busy &&
    data.items.every((record) => record.value !== null && !record.diagnostic)
  const refresh = () => setRevision((value) => value + 1)
  const run = (command: Command, label: string, after?: () => void) => {
    if (!editable || documentEditingSuspended(doc)) return
    setBusy(true)
    setFailed(false)
    void commitKerningGroup(doc, command, label)
      .then(after, (error) => {
        setFailed(true)
        reportError(error)
      })
      .finally(() => {
        setBusy(false)
        refresh()
      })
  }
  const maintain = (operation: KerningMaintenanceOperation, label: string) => {
    if (!master || !data) return
    run(
      {
        KerningMaintenance: {
          master,
          vertical,
          expected_generation: data.status.generation,
          operation,
        },
      },
      label,
    )
  }
  const changeSort = (column: KerningTableSort) => {
    setDescending(sort === column ? !descending : false)
    setSort(column)
    setPage(0)
  }
  const sideCell = (side: KernSide, position: KerningGroupSide) =>
    'group' in side ? (
      <button
        type="button"
        className={styles.group}
        disabled={!editable}
        aria-label={t('kerningTable.rename', { name: side.group })}
        onClick={() => setRename({ name: side.group, side: position })}
      >
        {side.group}
      </button>
    ) : (
      side.glyph
    )

  return (
    <details
      className={styles.panel}
      onToggle={(event) => setOpen(event.currentTarget.open)}
    >
      <summary>{t('kerningTable.title')}</summary>
      {open && (
        <div className={styles.body}>
          <div className={styles.controls}>
            <label>
              {t('kerningTable.filter')}
              <input
                value={query}
                onChange={(event) => {
                  setQuery(event.currentTarget.value)
                  setPage(0)
                }}
              />
            </label>
            <button type="button" disabled={busy} onClick={refresh}>
              {t('kerningTable.refresh')}
            </button>
            <button
              type="button"
              disabled={!editable}
              title={t('kerningTable.cleanupHint')}
              onClick={() =>
                maintain('CleanupInvalid', t('kerningTable.undoCleanup'))
              }
            >
              {t('kerningTable.cleanup')}
            </button>
            <button
              type="button"
              disabled={!editable}
              title={t('kerningTable.compressHint')}
              onClick={() =>
                maintain('Compress', t('kerningTable.undoCompress'))
              }
            >
              {t('kerningTable.compress')}
            </button>
            <button
              type="button"
              disabled={!canCopy}
              onClick={() => {
                if (master && data && canCopy)
                  setClipboard({
                    doc,
                    master,
                    vertical,
                    records: data.items.map((record) => ({
                      left: record.left,
                      right: record.right,
                      value: record.value!,
                    })),
                  })
              }}
            >
              {t('kerningTable.copy')}
            </button>
            <button
              type="button"
              disabled={!editable || !copied}
              onClick={() => {
                if (copied)
                  maintain(
                    { Paste: { records: copied.records, replace } },
                    t('kerningTable.undoPaste'),
                  )
              }}
            >
              {t('kerningTable.paste')}
            </button>
            <label>
              <input
                type="checkbox"
                checked={replace}
                disabled={busy}
                onChange={(event) => setReplace(event.currentTarget.checked)}
              />
              {t('kerningTable.replace')}
            </label>
          </div>
          {copied && (
            <p className={styles.hint}>
              {t('kerningTable.copied', {
                count: copied.records.length,
                master: copied.master,
              })}
            </p>
          )}
          {replace && (
            <p className={styles.hint}>{t('kerningTable.replaceHint')}</p>
          )}
          {!answer && master && (
            <p role="status">{t('kerningTable.loading')}</p>
          )}
          {(answer?.failed || failed) && (
            <p role="alert">{t('kerningTable.failed')}</p>
          )}
          {data?.status.reason && (
            <p role="status">{t(`kerningText.${data.status.reason}`)}</p>
          )}
          {data && (
            <>
              <div className={styles.scroll}>
                <table>
                  <thead>
                    <tr>
                      {(['left', 'right', 'value'] as const).map((column) => (
                        <th
                          key={column}
                          aria-sort={
                            sort === column
                              ? descending
                                ? 'descending'
                                : 'ascending'
                              : 'none'
                          }
                        >
                          <button
                            type="button"
                            onClick={() => changeSort(column)}
                          >
                            {t(`kerningTable.${column}`)}
                          </button>
                        </th>
                      ))}
                      <th>{t('kerningTable.actions')}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {records
                      .slice(
                        activePage * PAGE_SIZE,
                        (activePage + 1) * PAGE_SIZE,
                      )
                      .map((record) => (
                        <tr key={JSON.stringify([record.left, record.right])}>
                          <td>{sideCell(record.left, 'side1')}</td>
                          <td>{sideCell(record.right, 'side2')}</td>
                          <td
                            className={
                              record.value !== null && record.value < 0
                                ? styles.negative
                                : record.value !== null && record.value > 0
                                  ? styles.positive
                                  : undefined
                            }
                          >
                            {record.value ?? t('kerningTable.invalid')}
                            {record.diagnostic && (
                              <span title={record.diagnostic}>
                                {' '}
                                {t('kerningTable.invalid')}
                              </span>
                            )}
                          </td>
                          <td>
                            <button
                              type="button"
                              disabled={!editable || Boolean(record.diagnostic)}
                              aria-label={t('kerningTable.removePair', {
                                left: kerningSideName(record.left),
                                right: kerningSideName(record.right),
                              })}
                              onClick={() => {
                                if (master)
                                  run(
                                    vertical
                                      ? {
                                          SetVerticalKerning: {
                                            master,
                                            left: record.left,
                                            right: record.right,
                                            value: null,
                                          },
                                        }
                                      : {
                                          SetHorizontalKerning: {
                                            master,
                                            left: record.left,
                                            right: record.right,
                                            value: null,
                                          },
                                        },
                                    t('kerningText.undoRemove'),
                                  )
                              }}
                            >
                              {t('kerningTable.remove')}
                            </button>
                          </td>
                        </tr>
                      ))}
                  </tbody>
                </table>
              </div>
              <div className={styles.controls}>
                <button
                  type="button"
                  disabled={activePage === 0}
                  onClick={() => setPage(activePage - 1)}
                >
                  {t('kerningTable.previous')}
                </button>
                <span role="status">
                  {t('kerningTable.count', {
                    count: records.length,
                    total: data.total,
                    page: activePage + 1,
                    pages: maxPage + 1,
                  })}
                </span>
                <button
                  type="button"
                  disabled={activePage >= maxPage}
                  onClick={() => setPage(activePage + 1)}
                >
                  {t('kerningTable.next')}
                </button>
              </div>
            </>
          )}
        </div>
      )}
      {rename && (
        <KerningRenameDialog
          key={`${rename.side}:${rename.name}`}
          {...rename}
          busy={busy}
          onCancel={() => {
            if (!busy) setRename(null)
          }}
          onRename={(to) =>
            run(
              {
                RenameKerningGroup: {
                  side: rename.side,
                  from: rename.name,
                  to,
                },
              },
              t('kerningGroups.undo.rename'),
              () => setRename(null),
            )
          }
        />
      )}
    </details>
  )
}
