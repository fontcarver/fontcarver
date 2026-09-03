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
import { selectSummary, useSessionStore } from '../stores/sessionStore'
import { selectActiveTab, useUiStore } from '../stores/uiStore'
import { GlyphInspector } from '../views/glyph/GlyphInspector'

import styles from './panels.module.css'

const SORTS: readonly GlyphSort[] = ['Order', 'Unicode', 'Name', 'Width']
const LABEL_MODES: readonly LabelMode[] = ['name', 'unicode', 'index']

/**
 * Right pane: facts about the active document plus, on the Font tab only, the
 * glyph-grid display settings.
 */
export function Inspector() {
  const { t } = useTranslation()
  const tab = useUiStore(selectActiveTab)
  const summary = useSessionStore(selectSummary(tab?.doc ?? null))
  const view = useDocumentStore(selectDocView(tab?.doc ?? null))
  const setMaster = useDocumentStore((state) => state.setMaster)
  const setSort = useDocumentStore((state) => state.setSort)
  const setLabelMode = useDocumentStore((state) => state.setLabelMode)
  const setCellSize = useDocumentStore((state) => state.setCellSize)

  if (!summary || !tab) {
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
        {tab.kind === 'glyph' ? <GlyphInspector /> : null}
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

        {tab.kind === 'fontinfo' ? (
          <p className={styles.hintBlock}>{t('inspector.fontInfoNote')}</p>
        ) : null}
        {tab.kind === 'font' ? (
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
        ) : null}
      </div>
    </>
  )
}
