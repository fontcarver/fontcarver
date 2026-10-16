import { useCallback, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { DocSummary } from '../../api'
import { withSettledDocument } from '../../editor/documentPersistence'
import { reportError } from '../../shell/commands'
import { selectDocView, useDocumentStore } from '../../stores/documentStore'
import { FeatureMasterNavigation } from './FeatureMasterNavigation'
import { FeatureSourceEditor } from './FeatureSourceEditor'
import { FeatureDiagnostics } from './FeatureDiagnostics'
import { FeatureGenerationPanel } from './FeatureGenerationPanel'
import { FeatureSourceTree } from './FeatureSourceTree'
import { LookupInspector } from './LookupInspector'
import { VisualLookupEditor } from './VisualLookupEditor'
import {
  addSource,
  removeSource,
  sourceItems,
  type SourceKind,
} from './featureSources'
import { useFeatureCompletions } from './useFeatureCompletions'
import { useFeaturesDraft } from './useFeaturesDraft'
import styles from './FeaturesView.module.css'

/** Document source editor keeps drafts, accepted source and imported authority distinct. */
export function FeaturesView({ summary }: { summary: DocSummary }) {
  const { t } = useTranslation()
  const view = useDocumentStore(selectDocView(summary.doc))
  const navigation = useMemo(
    () => new FeatureMasterNavigation(summary.doc),
    [summary.doc],
  )
  const master = view.master ?? summary.masters[0]?.id ?? null
  const draft = useFeaturesDraft(summary.doc, view.generation, master)
  const {
    names: glyphNames,
    loadGlyphNames,
    completionKey,
  } = useFeatureCompletions(summary.doc, view.generation)
  const [selected, setSelected] = useState<string | null>(null)
  const [mode, setMode] = useState<'source' | 'visual' | 'lookups'>('source')
  const [focusRange, setFocusRange] = useState<{
    from: number
    to: number
    revision: number
  } | null>(null)
  const { source, settings, state, session } = draft
  const action = useCallback(
    (run: () => Promise<void>) => {
      void withSettledDocument(summary.doc, run).catch(reportError)
    },
    [summary.doc],
  )
  const refresh = useCallback(() => {
    void session.reload().catch(reportError)
  }, [session])
  const select = (id: string) => {
    void session
      .flush()
      .then(() => {
        setSelected(id)
        setFocusRange(null)
      })
      .catch(reportError)
  }
  if (!source || !settings || !state)
    return (
      <section className={styles.root} aria-label={t('features.title')}>
        <p className={styles.empty}>
          {t(draft.error ? 'features.loadError' : 'features.loading')}
        </p>
        {!!draft.error && (
          <button type="button" onClick={refresh}>
            {t('features.retry')}
          </button>
        )}
      </section>
    )
  const imported = state.authority === 'ImportedTables'
  const readOnly = imported || draft.busy || draft.suspended
  const previewSource = draft.validation?.generated_source ?? source
  const visibleSettings = draft.validation?.generated_settings ?? settings
  const canFreeze =
    JSON.stringify([source, settings]) ===
    JSON.stringify([previewSource, visibleSettings])
  // A draft edit made after the last check adds or renames entries the
  // generated preview does not know about yet. They stay visible and
  // selectable until the next check refreshes the preview.
  const previewIds = new Set(sourceItems(previewSource).map((item) => item.id))
  const visibleSource = canFreeze
    ? previewSource
    : {
        ...previewSource,
        prefixes: [
          ...previewSource.prefixes,
          ...source.prefixes.filter((block) => !previewIds.has(block.id)),
        ],
        classes: [
          ...previewSource.classes,
          ...source.classes.filter((block) => !previewIds.has(block.id)),
        ],
        features: [
          ...previewSource.features,
          ...source.features.filter((block) => !previewIds.has(block.id)),
        ],
      }
  const items = sourceItems(visibleSource)
  const item = items.find((candidate) => candidate.id === selected) ?? items[0]
  const generated = visibleSettings.generated.find(
    (record) => record.source_id === item?.id,
  )
  const status = draft.busy
    ? 'checking'
    : draft.saving
      ? 'saving'
      : draft.pending
        ? 'unsavedDraft'
        : imported
          ? 'original'
          : state.has_unapplied_changes
            ? 'savedDraft'
            : 'applied'
  const add = (kind: SourceKind) => {
    void session
      .flush()
      .then(() => {
        const snapshot = session.getSnapshot()
        if (!snapshot.source || !snapshot.settings) return
        const label = t(
          kind === 'prefixes'
            ? 'features.newPrefix'
            : kind === 'classes'
              ? 'features.newClass'
              : 'features.newFeature',
        )
        const next = addSource(snapshot.source, kind, label)
        session.change(next, snapshot.settings)
        setSelected(next[kind].at(-1)!.id)
        setFocusRange(null)
      })
      .catch(reportError)
  }
  const remove = (id: string) => {
    void session
      .flush()
      .then(() => {
        const snapshot = session.getSnapshot()
        if (snapshot.source && snapshot.settings)
          session.change(removeSource(snapshot.source, id), snapshot.settings)
      })
      .catch(reportError)
  }
  const navigate = (id: string, from: number, to: number) => {
    setSelected(id)
    setFocusRange({ from, to, revision: (focusRange?.revision ?? 0) + 1 })
  }
  return (
    <section className={styles.root} aria-label={t('features.title')}>
      <header className={styles.toolbar}>
        <h2>{t('features.title')}</h2>
        <div className={styles.mode} aria-label={t('features.title')}>
          <button
            type="button"
            aria-pressed={mode === 'source'}
            onClick={() => {
              void session
                .flush()
                .then(() => setMode('source'))
                .catch(reportError)
            }}
          >
            {t('features.source')}
          </button>
          <button
            type="button"
            aria-pressed={mode === 'visual'}
            onClick={() => {
              void session
                .flush()
                .then(() => setMode('visual'))
                .catch(reportError)
            }}
          >
            {t('visualLookup.mode')}
          </button>
          <button
            type="button"
            aria-pressed={mode === 'lookups'}
            onClick={() => {
              void session
                .flush()
                .then(() => setMode('lookups'))
                .catch(reportError)
            }}
          >
            {t('features.lookups')}
          </button>
        </div>
        <button
          type="button"
          className={styles.primary}
          disabled={readOnly}
          onClick={() => action(session.apply)}
        >
          {t('features.compile')}
        </button>
        <button
          type="button"
          disabled={readOnly}
          onClick={() => action(session.validate)}
        >
          {t('features.validate')}
        </button>
        <button
          type="button"
          disabled={
            readOnly || (!draft.pending && !state.has_unapplied_changes)
          }
          onClick={() => action(session.discard)}
        >
          {t('features.discard')}
        </button>
        {summary.masters.length > 1 && (
          <label className={styles.master}>
            {t('features.master')}
            <select
              value={master ?? ''}
              onChange={(event) => {
                const next = event.target.value
                void navigation.change(next).catch(reportError)
              }}
            >
              {summary.masters.map((entry) => (
                <option key={entry.id} value={entry.id}>
                  {entry.name}
                </option>
              ))}
            </select>
          </label>
        )}
        <span role="status" className={styles.status}>
          {t(`features.${status}`)}
        </span>
      </header>
      <div className={styles.notice}>
        <p>{t(imported ? 'features.importedHelp' : 'features.draftHelp')}</p>
        {state.has_unapplied_changes && <p>{t('features.unappliedExport')}</p>}
      </div>
      {!!draft.error && (
        <div role="alert" className={`${styles.notice} ${styles.error}`}>
          <p>
            {t(
              draft.error instanceof Error &&
                'code' in draft.error &&
                draft.error.code === 'stale_features'
                ? 'features.conflict'
                : draft.pending
                  ? 'features.persistenceFailed'
                  : 'features.loadError',
            )}
          </p>
          <details>
            <summary>{t('features.details')}</summary>
            {draft.error instanceof Error
              ? draft.error.message
              : String(draft.error)}
          </details>
          <button
            type="button"
            onClick={() =>
              void (draft.pending ? session.flush() : session.reload()).catch(
                reportError,
              )
            }
          >
            {t('features.retry')}
          </button>
          {draft.pending && (
            <button
              type="button"
              onClick={() => void session.recover().catch(reportError)}
            >
              {t('features.recover')}
            </button>
          )}
        </div>
      )}
      {mode === 'visual' ? (
        <>
          <VisualLookupEditor
            source={source}
            selected={selected}
            onSelect={setSelected}
            master={master}
            generation={state.compile_revision}
            session={session}
            readOnly={readOnly}
            onSource={(id) => {
              void session
                .flush()
                .then(() => {
                  setSelected(id)
                  setMode('source')
                })
                .catch(reportError)
            }}
          />
          <FeatureDiagnostics
            validation={draft.validation}
            source={visibleSource}
            onNavigate={(id, from, to) => {
              navigate(id, from, to)
              setMode('source')
            }}
          />
        </>
      ) : mode === 'lookups' ? (
        <LookupInspector
          doc={summary.doc}
          generation={view.generation}
          master={master}
          onChanged={refresh}
          beforeChange={session.flush}
        />
      ) : (
        <div className={styles.workspace}>
          <FeatureSourceTree
            source={visibleSource}
            settings={visibleSettings}
            selected={item?.id ?? null}
            readOnly={readOnly}
            onSelect={select}
            onAdd={add}
          />
          <div className={styles.editorColumn}>
            {item ? (
              <>
                <FeatureSourceEditor
                  source={source}
                  previewSource={visibleSource}
                  master={master}
                  generation={view.generation}
                  item={item}
                  settings={settings}
                  automaticEntries={state.automatic_entries}
                  generated={generated}
                  generatedSaved={
                    canFreeze &&
                    !!generated &&
                    settings.generated.some(
                      (record) => record.source_id === generated.source_id,
                    )
                  }
                  readOnly={readOnly}
                  glyphNames={glyphNames}
                  loadGlyphNames={loadGlyphNames}
                  completionKey={completionKey}
                  diagnostics={draft.validation?.diagnostics ?? []}
                  focusRange={focusRange}
                  session={session}
                  onRemove={remove}
                />
              </>
            ) : (
              <p className={styles.empty}>{t('features.noSelection')}</p>
            )}
            <FeatureDiagnostics
              validation={draft.validation}
              source={visibleSource}
              onNavigate={navigate}
            />
          </div>
          <FeatureGenerationPanel
            settings={settings}
            classRecipes={state.automatic_entries
              .filter((entry) => entry.kind === 'Class')
              .map((entry) => entry.name)}
            canFreeze={canFreeze}
            readOnly={readOnly}
            stale={state.generation_stale}
            onChange={(next) => session.change(source, next)}
            onFreeze={(generator) => action(() => session.freeze(generator))}
          />
        </div>
      )}
    </section>
  )
}
