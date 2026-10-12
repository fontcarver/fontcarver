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
  DEFAULT_KERNING_FILTER,
  type KerningMatchMode,
  type KerningRecordKind,
  type KerningTableFilter,
  type KerningTableSort,
  visibleKerningRecords,
} from './kerningTableData'
import { useKerningTable } from './useKerningTable'
import styles from './KerningTable.module.css'

const PAGE_SIZE = 50
const MATCH_MODES: readonly KerningMatchMode[] = [
  'contains',
  'is',
  'starts',
  'ends',
  'excludes',
]
const RECORD_KINDS: readonly KerningRecordKind[] = [
  'all',
  'groups',
  'glyphs',
  'exceptions',
]

/** How a pasted table resolves a key that already exists in the destination. */
type PasteMode = 'overwrite' | 'keep' | 'replace'

/** One master the table can inspect, independently of the text run's master. */
export interface KerningTableMaster {
  id: string
  name: string
}

/**
 * Stored pair management for one master and direction.
 *
 * The table inspects a master of its own choosing rather than only the one the
 * proof is set to, which is what makes copy/paste *between masters* a two-click
 * operation instead of a round trip through the text bar. Group sides open the
 * rename dialog, exceptions are marked, and the maintenance commands all carry
 * the generation the visible table was read at.
 */
export function KerningTable({
  doc,
  master,
  masters,
  generation,
  vertical,
  disabled = false,
}: {
  doc: DocId
  /** The text run's master, used as the table's initial choice. */
  master: string | null
  /** Every master of the document, for the table's own master control. */
  masters?: readonly KerningTableMaster[]
  generation: number
  vertical: boolean
  disabled?: boolean
}) {
  const { t } = useTranslation()
  const [open, setOpen] = useState(false)
  const [revision, setRevision] = useState(0)
  const [filter, setFilter] = useState<KerningTableFilter>(
    DEFAULT_KERNING_FILTER,
  )
  const [sort, setSort] = useState<KerningTableSort>('left')
  const [descending, setDescending] = useState(false)
  const [page, setPage] = useState(0)
  const [busy, setBusy] = useState(false)
  const [failed, setFailed] = useState(false)
  const [pasteMode, setPasteMode] = useState<PasteMode>('overwrite')
  const [chosenMaster, setChosenMaster] = useState<string | null>(null)
  const [clipboard, setClipboard] = useState<{
    doc: DocId
    master: string
    vertical: boolean
    records: KernPair[]
  } | null>(null)
  const [rename, setRename] = useState<{
    name: string
    side: KerningGroupSide
    record: { left: KernSide; right: KernSide; value: number | null }
  } | null>(null)
  // A master the document no longer has must not strand the table.
  const known = (id: string | null): string | null =>
    id !== null && (!masters || masters.some((entry) => entry.id === id))
      ? id
      : null
  const shown = known(chosenMaster) ?? master
  const answer = useKerningTable(
    doc,
    shown,
    vertical,
    generation,
    open,
    revision,
  )
  const data = answer?.data
  const records = useMemo(
    () => visibleKerningRecords(data?.items ?? [], filter, sort, descending),
    [data, filter, sort, descending],
  )
  const maxPage = Math.max(0, Math.ceil(records.length / PAGE_SIZE) - 1)
  const activePage = Math.min(page, maxPage)
  const editable = Boolean(data?.status.editable && shown && !busy && !disabled)
  const copied =
    clipboard?.doc === doc && clipboard.vertical === vertical ? clipboard : null
  const canCopy =
    data &&
    !busy &&
    data.items.every((record) => record.value !== null && !record.diagnostic)
  const refresh = () => setRevision((value) => value + 1)
  const patchFilter = (changes: Partial<KerningTableFilter>) => {
    setFilter((current) => ({ ...current, ...changes }))
    setPage(0)
  }
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
    if (!shown || !data) return
    run(
      {
        KerningMaintenance: {
          master: shown,
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
  const sideCell = (
    side: KernSide,
    position: KerningGroupSide,
    record: { left: KernSide; right: KernSide; value: number | null },
  ) =>
    'group' in side ? (
      <button
        type="button"
        className={styles.group}
        disabled={!editable}
        aria-label={t('kerningTable.rename', { name: side.group })}
        onClick={() => setRename({ name: side.group, side: position, record })}
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
            {masters && masters.length > 1 && (
              <span className={styles.field}>
                {t('kerningTable.master')}
                <select
                  value={shown ?? ''}
                  disabled={busy}
                  aria-label={t('kerningTable.master')}
                  onChange={(event) => {
                    setChosenMaster(event.currentTarget.value)
                    setPage(0)
                  }}
                >
                  {masters.map((entry) => (
                    <option key={entry.id} value={entry.id}>
                      {entry.name}
                    </option>
                  ))}
                </select>
              </span>
            )}
            <label>
              {t('kerningTable.filter')}
              <input
                value={filter.query}
                onChange={(event) =>
                  patchFilter({ query: event.currentTarget.value })
                }
              />
            </label>
            <select
              value={filter.match}
              aria-label={t('kerningTable.matchMode')}
              onChange={(event) =>
                patchFilter({
                  match: event.currentTarget.value as KerningMatchMode,
                })
              }
            >
              {MATCH_MODES.map((mode) => (
                <option key={mode} value={mode}>
                  {t(`kerningTable.match.${mode}`)}
                </option>
              ))}
            </select>
            <select
              value={filter.kind}
              aria-label={t('kerningTable.kind')}
              onChange={(event) =>
                patchFilter({
                  kind: event.currentTarget.value as KerningRecordKind,
                })
              }
            >
              {RECORD_KINDS.map((kind) => (
                <option key={kind} value={kind}>
                  {t(`kerningTable.kinds.${kind}`)}
                </option>
              ))}
            </select>
            <button type="button" disabled={busy} onClick={refresh}>
              {t('kerningTable.refresh')}
            </button>
            <button
              type="button"
              disabled={!editable}
              title={t('kerningTable.cleanupHint')}
              onClick={() =>
                maintain('CleanupObsolete', t('kerningTable.undoCleanup'))
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
                if (shown && data && canCopy)
                  setClipboard({
                    doc,
                    master: shown,
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
                if (!copied) return
                maintain(
                  pasteMode === 'keep'
                    ? { PasteKeepingExisting: { records: copied.records } }
                    : {
                        Paste: {
                          records: copied.records,
                          replace: pasteMode === 'replace',
                        },
                      },
                  t('kerningTable.undoPaste'),
                )
              }}
            >
              {t('kerningTable.paste')}
            </button>
            <span className={styles.field}>
              {t('kerningTable.conflict')}
              <select
                value={pasteMode}
                disabled={busy}
                aria-label={t('kerningTable.conflict')}
                onChange={(event) =>
                  setPasteMode(event.currentTarget.value as PasteMode)
                }
              >
                {(['overwrite', 'keep', 'replace'] as const).map((mode) => (
                  <option key={mode} value={mode}>
                    {t(`kerningTable.conflicts.${mode}`)}
                  </option>
                ))}
              </select>
            </span>
          </div>
          {copied && (
            <p className={styles.hint}>
              {t('kerningTable.copied', {
                count: copied.records.length,
                master: copied.master,
              })}
            </p>
          )}
          {pasteMode === 'replace' && (
            <p className={styles.hint}>{t('kerningTable.replaceHint')}</p>
          )}
          {!answer && shown && <p role="status">{t('kerningTable.loading')}</p>}
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
                      <th>{t('kerningTable.scope')}</th>
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
                          <td>{sideCell(record.left, 'side1', record)}</td>
                          <td>{sideCell(record.right, 'side2', record)}</td>
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
                            {record.exception
                              ? t('kerningTable.exception')
                              : 'group' in record.left ||
                                  'group' in record.right
                                ? t('kerningTable.classRule')
                                : t('kerningTable.pairRule')}
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
                                if (shown)
                                  run(
                                    removeCommand(
                                      shown,
                                      vertical,
                                      record.left,
                                      record.right,
                                    ),
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
          name={rename.name}
          side={rename.side}
          doc={doc}
          busy={busy}
          canRetarget={rename.record.value !== null}
          onCancel={() => {
            if (!busy) setRename(null)
          }}
          onRename={(to, scope) => {
            if (scope === 'font') {
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
              return
            }
            // "This record only" retargets the one row: the old record is
            // removed and an equivalent one is stored against the new group,
            // leaving the group itself and every other pair untouched. The
            // dialog disables this scope without a readable value, so reaching
            // here with one is a bug, not a user path.
            if (!shown || rename.record.value === null) return
            const side: KernSide = { group: to }
            run(
              {
                Batch: [
                  removeCommand(
                    shown,
                    vertical,
                    rename.record.left,
                    rename.record.right,
                  ),
                  setCommand(
                    shown,
                    vertical,
                    rename.side === 'side1' ? side : rename.record.left,
                    rename.side === 'side2' ? side : rename.record.right,
                    rename.record.value,
                  ),
                ],
              },
              t('kerningTable.undoRetarget'),
              () => setRename(null),
            )
          }}
        />
      )}
    </details>
  )
}

function removeCommand(
  master: string,
  vertical: boolean,
  left: KernSide,
  right: KernSide,
): Command {
  return vertical
    ? { SetVerticalKerning: { master, left, right, value: null } }
    : { SetHorizontalKerning: { master, left, right, value: null } }
}

function setCommand(
  master: string,
  vertical: boolean,
  left: KernSide,
  right: KernSide,
  value: number,
): Command {
  return vertical
    ? { SetVerticalKerning: { master, left, right, value } }
    : { SetHorizontalKerning: { master, left, right, value } }
}
