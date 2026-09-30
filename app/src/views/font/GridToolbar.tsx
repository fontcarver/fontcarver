import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { DocSummary, GlyphSort } from '../../api'
import { Button } from '../../components/Button'
import { Select } from '../../components/Select'
import { Slider } from '../../components/Slider'
import { Toolbar } from '../../components/Toolbar'
import {
  CELL_SIZE_MAX,
  CELL_SIZE_MIN,
  useDocumentStore,
  type DocView,
  type LabelMode,
} from '../../stores/documentStore'

import styles from './fontView.module.css'
import { CjkBatchDialog } from './CjkBatchDialog'
import { OpticalSpacingDialog } from './OpticalSpacingDialog'
import { GlyphSearch } from './GlyphSearch'
import { useUiStore } from '../../stores/uiStore'

const SORTS: readonly GlyphSort[] = [
  'Order',
  'Unicode',
  'Name',
  'Width',
  'Radical',
  'Strokes',
  'Reading',
  'Cid',
]
const LABEL_MODES: readonly LabelMode[] = ['name', 'unicode', 'index']

/** The whole Unicode range; used to express "encoded glyphs only". */
const ANY_UNICODE: [number, number] = [0, 0x10ffff]

export interface GridToolbarProps {
  summary: DocSummary
  view: DocView
}

/** Display controls above the glyph grid, plus the search field. */
export function GridToolbar({ summary, view }: GridToolbarProps) {
  const { t } = useTranslation()
  const [batch, setBatch] = useState<number[] | null>(null)
  const [optical, setOptical] = useState<{
    doc: number
    glyphs: number[]
    master: string
  } | null>(null)
  const doc = summary.doc
  const setViewMode = useDocumentStore((state) => state.setViewMode)
  const setMaster = useDocumentStore((state) => state.setMaster)
  const setSort = useDocumentStore((state) => state.setSort)
  const setLabelMode = useDocumentStore((state) => state.setLabelMode)
  const setCellSize = useDocumentStore((state) => state.setCellSize)
  const setFilter = useDocumentStore((state) => state.setFilter)

  const masterId = view.master ?? summary.masters[0]?.id ?? ''
  const encodedOnly = view.filter.unicode_ranges.length > 0
  const shown = view.matchCount ?? summary.glyph_count

  return (
    <>
      {optical && (
        <OpticalSpacingDialog {...optical} onClose={() => setOptical(null)} />
      )}
      {batch && (
        <CjkBatchDialog
          doc={doc}
          glyphs={batch}
          master={masterId}
          upem={summary.units_per_em}
          onClose={() => setBatch(null)}
        />
      )}
      <Toolbar label={t('fontView.toolbar')} className={styles.toolbarRow}>
        <span className={styles.toolbarGroup}>
          <Button
            aria-pressed={view.viewMode === 'grid'}
            onClick={() => setViewMode(doc, 'grid')}
          >
            {t('fontView.gridView')}
          </Button>
          <Button
            aria-pressed={view.viewMode === 'list'}
            onClick={() => setViewMode(doc, 'list')}
          >
            {t('fontView.listView')}
          </Button>
        </span>
        <span className={styles.toolbarGroup}>
          <span className={styles.toolbarLabel}>{t('inspector.master')}</span>
          <span className={styles.toolbarSelect}>
            <Select
              label={t('inspector.master')}
              value={masterId}
              disabled={summary.masters.length < 2}
              options={summary.masters.map((master) => ({
                value: master.id,
                label: master.name,
              }))}
              onChange={(value) => setMaster(doc, value)}
            />
          </span>
        </span>

        <span className={styles.toolbarGroup}>
          <span className={styles.toolbarLabel}>{t('inspector.sort')}</span>
          <span className={styles.toolbarSelect}>
            <Select
              label={t('inspector.sort')}
              value={view.sort}
              options={SORTS.map((sort) => ({
                value: sort,
                label: t(`inspector.sortMode.${sort}`),
              }))}
              onChange={(value) => setSort(doc, value)}
            />
          </span>
        </span>

        <span className={styles.toolbarGroup}>
          <span className={styles.toolbarLabel}>{t('inspector.labels')}</span>
          <span className={styles.toolbarSelect}>
            <Select
              label={t('inspector.labels')}
              value={view.labelMode}
              options={LABEL_MODES.map((mode) => ({
                value: mode,
                label: t(`inspector.labelMode.${mode}`),
              }))}
              onChange={(value) => setLabelMode(doc, value)}
            />
          </span>
        </span>

        <Button
          aria-pressed={encodedOnly}
          onClick={() =>
            setFilter(doc, {
              unicode_ranges: encodedOnly ? [] : [ANY_UNICODE],
              unencoded_only: false,
            })
          }
        >
          {t('fontView.encodedOnly')}
        </Button>

        <span className={styles.toolbarGroup}>
          <span className={styles.toolbarSlider}>
            <Slider
              label={t('inspector.cellSize')}
              min={CELL_SIZE_MIN}
              max={CELL_SIZE_MAX}
              value={view.cellSize}
              onChange={(value) => setCellSize(doc, value)}
            />
          </span>
          <span className={styles.toolbarCount}>{view.cellSize} px</span>
        </span>

        <Button
          disabled={view.selection.size === 0}
          onClick={() => setBatch([...view.selection])}
        >
          {t('cjk.batchTitle')}
        </Button>
        <Button
          disabled={view.selection.size === 0 || view.selection.size > 128}
          onClick={() =>
            setOptical({ doc, master: masterId, glyphs: [...view.selection] })
          }
        >
          {t('opticalSpacing.title')}
        </Button>
        <span className={styles.toolbarSpacer} />
        <Button
          disabled={view.selection.size === 0 || view.selection.size > 512}
          onClick={() =>
            useUiStore.getState().setDialog({
              kind: 'styleEffects',
              doc,
              glyphs: [...view.selection],
              master: masterId,
            })
          }
        >
          {t('styleEffects.title')}
        </Button>
        <span className={styles.toolbarCount}>
          {t('status.glyphs', { count: shown })}
        </span>
        <GlyphSearch
          doc={doc}
          value={view.filter.query}
          queryScope={view.filter.query_scope ?? 'All'}
          caseSensitive={view.filter.case_sensitive ?? false}
          regex={view.filter.regex ?? false}
        />
      </Toolbar>
    </>
  )
}
