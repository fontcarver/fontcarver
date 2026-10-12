import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type {
  AutomaticEntryOption,
  DocId,
  FeatureDiagnostic,
  FeatureGenerationSettings,
  Features,
  GeneratedFeatureRecord,
  MasterId,
} from '../../api'
import { withSettledDocument } from '../../editor/documentPersistence'
import { reportError, runCommand } from '../../shell/commands'
import {
  FeatureCodeEditor,
  type FeatureCodeEditorProps,
} from './FeatureCodeEditor'
import { type FeatureDraftSession } from './FeatureDraftSession'
import { FeatureClassPreview } from './FeatureClassPreview'
import {
  automaticEntryOption,
  switchEntryToAutomatic,
} from './automaticEntries'
import { FeatureAutomaticSwitch } from './FeatureAutomaticSwitch'
import { moveSource, updateSource, type SourceItem } from './featureSources'
import styles from './FeaturesView.module.css'

/** Source-local controls and the CodeMirror view share the document draft owner. */
export function FeatureSourceEditor({
  source,
  previewSource = source,
  item,
  settings,
  automaticEntries,
  generated,
  generatedSaved,
  readOnly,
  glyphNames,
  loadGlyphNames,
  diagnostics,
  focusRange,
  session,
  master = null,
  generation = 0,
  onRemove,
}: {
  source: Features
  previewSource?: Features
  item: SourceItem
  settings: FeatureGenerationSettings
  automaticEntries: readonly AutomaticEntryOption[]
  generated: GeneratedFeatureRecord | undefined
  generatedSaved: boolean
  readOnly: boolean
  glyphNames: readonly string[]
  loadGlyphNames: (query: string) => Promise<readonly string[]>
  diagnostics: readonly FeatureDiagnostic[]
  focusRange: FeatureCodeEditorProps['focusRange']
  session: FeatureDraftSession
  master?: MasterId | null
  generation?: number
  onRemove: (id: string) => void
}) {
  const { t } = useTranslation()
  const [classPreview, setClassPreview] = useState<{
    doc: DocId
    sourceId: string
    name: string
  } | null>(null)
  const change = (next: Features) => {
    const settings = session.getSnapshot().settings
    if (settings) session.change(next, settings)
  }
  const action = (run: () => Promise<void>) => {
    void withSettledDocument(session.doc, run).catch(reportError)
  }
  return (
    <>
      <div className={styles.sourceToolbar}>
        <label>
          {t(
            item.kind === 'classes'
              ? 'features.className'
              : item.kind === 'features'
                ? 'features.featureTag'
                : 'features.prefixName',
          )}
          <input
            disabled={readOnly || !!generated}
            value={item.label}
            onChange={(event) =>
              change(
                updateSource(
                  source,
                  item.id,
                  item.kind === 'classes'
                    ? { name: event.target.value }
                    : { tag: event.target.value },
                ),
              )
            }
          />
        </label>
        {item.kind !== 'classes' && (
          <label>
            <input
              type="checkbox"
              disabled={readOnly || !!generated}
              checked={!item.disabled}
              onChange={(event) =>
                change(
                  updateSource(source, item.id, {
                    disabled: !event.target.checked,
                  }),
                )
              }
            />
            {t('features.enableSource')}
          </label>
        )}
        <button
          type="button"
          disabled={
            readOnly || !!generated || source[item.kind][0]?.id === item.id
          }
          aria-label={t('features.moveUp')}
          onClick={() => change(moveSource(source, item.id, -1))}
        >
          ↑
        </button>
        <button
          type="button"
          disabled={
            readOnly || !!generated || source[item.kind].at(-1)?.id === item.id
          }
          aria-label={t('features.moveDown')}
          onClick={() => change(moveSource(source, item.id, 1))}
        >
          ↓
        </button>
        <button
          type="button"
          disabled={readOnly || !!generated}
          onClick={() => onRemove(item.id)}
        >
          {t('features.removeSource')}
        </button>
        <FeatureAutomaticSwitch
          item={item}
          option={automaticEntryOption(automaticEntries, item)}
          settings={settings}
          generated={generated}
          generatedSaved={generatedSaved}
          readOnly={readOnly}
          onManual={(option) =>
            action(() => session.freeze(option.generator, option.name))
          }
          onAutomatic={(option) => {
            const snapshot = session.getSnapshot()
            if (!snapshot.source || !snapshot.settings) return
            const next = switchEntryToAutomatic(
              snapshot.source,
              snapshot.settings,
              item,
              option,
              automaticEntries,
            )
            session.change(next.source, next.settings)
          }}
        />
      </div>
      {generated && (
        <div className={styles.generatedInfo}>
          <p>
            {t('features.generatedBy', {
              generator: t(`features.generator.${generated.generator}`),
            })}
          </p>
          <p>
            {t('features.generatedGlyphs', {
              glyphs: generated.glyphs.slice(0, 32).join(', '),
            })}
          </p>
          <button
            type="button"
            disabled={readOnly || !generatedSaved}
            title={t('features.freezePreview')}
            onClick={() => action(() => session.freeze(generated.generator))}
          >
            {t('features.freeze')}
          </button>
        </div>
      )}
      <div className={styles.editor}>
        <FeatureCodeEditor
          key={item.id}
          value={item.code}
          readOnly={readOnly || !!generated}
          label={t('features.sourceCode')}
          glyphNames={glyphNames}
          loadGlyphNames={loadGlyphNames}
          classNames={previewSource.classes.map((entry) => entry.name)}
          diagnostics={diagnostics
            .filter((entry) => entry.source_id === item.id && entry.span)
            .map((entry) => ({
              from: entry.span!.start,
              to: entry.span!.end,
              severity: entry.severity.toLowerCase() as
                'error' | 'warning' | 'info',
              message: entry.message,
            }))}
          focusRange={focusRange}
          phrases={{
            Completions: t('features.completionLabel'),
            Diagnostics: t('features.diagnostics'),
            'No diagnostics': t('features.noDiagnostics'),
            close: t('common.close'),
            'Control character': t('features.controlCharacter'),
          }}
          onChange={(code) => change(updateSource(source, item.id, { code }))}
          onCompositionChange={session.setComposing}
          onHistory={(direction) => runCommand(`edit.${direction}`)}
          onSave={() => runCommand('file.save')}
          onClassPreview={(name) =>
            setClassPreview({ doc: session.doc, sourceId: item.id, name })
          }
        />
      </div>
      <p className={styles.muted}>{t('features.classPreview.help')}</p>
      {classPreview?.doc === session.doc &&
        classPreview.sourceId === item.id && (
          <FeatureClassPreview
            doc={session.doc}
            master={master}
            generation={generation}
            source={previewSource}
            name={classPreview.name}
            onClose={() => setClassPreview(null)}
          />
        )}
    </>
  )
}
