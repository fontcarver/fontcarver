import { useTranslation } from 'react-i18next'

import type { GlyphSort } from '../api'
import { Field } from '../components/Field'
import { Panel } from '../components/Panel'
import { Select } from '../components/Select'
import { Slider } from '../components/Slider'
import {
  CELL_SIZE_MAX,
  CELL_SIZE_MIN,
  selectDocView,
  useDocumentStore,
  type LabelMode,
} from '../stores/documentStore'
import { selectActiveSummary, useSessionStore } from '../stores/sessionStore'
import { selectActiveTab, useUiStore } from '../stores/uiStore'

import styles from './shell.module.css'

const SORTS: readonly GlyphSort[] = ['Order', 'Unicode', 'Name', 'Width']
const LABEL_MODES: readonly LabelMode[] = ['name', 'unicode', 'index']

/** Right inspector: document facts plus the font view's display settings. */
export function Inspector() {
  const { t } = useTranslation()
  const summary = useSessionStore(selectActiveSummary)
  const activeTab = useUiStore(selectActiveTab)
  const view = useDocumentStore(selectDocView(summary?.doc ?? null))
  const setMaster = useDocumentStore((state) => state.setMaster)
  const setSort = useDocumentStore((state) => state.setSort)
  const setLabelMode = useDocumentStore((state) => state.setLabelMode)
  const setCellSize = useDocumentStore((state) => state.setCellSize)

  if (!summary) {
    return (
      <>
        <div className={styles.panelTitle}>{t('inspector.title')}</div>
        <p className={styles.hintBlock}>{t('inspector.empty')}</p>
      </>
    )
  }

  const doc = summary.doc
  const masterId =
    view.master?.toString() ?? summary.masters[0]?.id.toString() ?? ''

  return (
    <>
      <div className={styles.panelTitle}>{t('inspector.title')}</div>
      <div className={styles.panelStack}>
        <Panel title={t('inspector.document')}>
          <Field label={t('inspector.family')} readOnly>
            {summary.family_name}
          </Field>
          <Field label={t('inspector.style')} readOnly>
            {summary.style_name}
          </Field>
          <Field label={t('inspector.format')} readOnly>
            {summary.format}
          </Field>
          <Field label={t('inspector.unitsPerEm')} readOnly>
            {summary.units_per_em}
          </Field>
          <Field label={t('inspector.glyphs')} readOnly>
            {summary.glyph_count.toLocaleString()}
          </Field>
          <Field label={t('inspector.masters')} readOnly>
            {summary.masters.length}
          </Field>
          <Field label={t('inspector.axes')} readOnly>
            {summary.axes.length}
          </Field>
          <Field label={t('inspector.path')} readOnly>
            {summary.path ?? '—'}
          </Field>
        </Panel>

        {activeTab?.kind === 'glyph' ? (
          <p className={styles.hintBlock}>{t('inspector.glyphNote')}</p>
        ) : (
          <Panel title={t('inspector.view')}>
            <Field label={t('inspector.master')}>
              <Select
                label={t('inspector.master')}
                value={masterId}
                options={summary.masters.map((master) => ({
                  value: master.id.toString(),
                  label: master.name,
                }))}
                onChange={(value) => setMaster(doc, value)}
              />
            </Field>
            <Field label={t('inspector.sort')}>
              <Select
                label={t('inspector.sort')}
                value={view.sort}
                options={SORTS.map((sort) => ({
                  value: sort,
                  label: t(`inspector.sortMode.${sort}`),
                }))}
                onChange={(value) => setSort(doc, value)}
              />
            </Field>
            <Field label={t('inspector.labels')}>
              <Select
                label={t('inspector.labels')}
                value={view.labelMode}
                options={LABEL_MODES.map((mode) => ({
                  value: mode,
                  label: t(`inspector.labelMode.${mode}`),
                }))}
                onChange={(value) => setLabelMode(doc, value)}
              />
            </Field>
            <Field label={t('inspector.cellSize')}>
              <Slider
                label={t('inspector.cellSize')}
                min={CELL_SIZE_MIN}
                max={CELL_SIZE_MAX}
                value={view.cellSize}
                valueLabel={`${view.cellSize} px`}
                onChange={(value) => setCellSize(doc, value)}
              />
            </Field>
          </Panel>
        )}
      </div>
    </>
  )
}
