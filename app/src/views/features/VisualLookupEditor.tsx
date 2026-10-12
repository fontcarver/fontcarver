import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { Features } from '../../api'
import { type FeatureDraftSession } from './FeatureDraftSession'
import { addSource, removeSource, updateSource } from './featureSources'
import { VisualLookupFields } from './VisualLookupFields'
import { VisualLookupPreview } from './VisualLookupPreview'
import {
  defaultVisualLookup,
  isValidFeatureTag,
  parseVisualLookup,
  serializeVisualLookup,
  validateVisualLookup,
  VISUAL_LOOKUP_KINDS,
  type VisualLookup,
  type VisualLookupKind,
} from './visualLookup'
import styles from './VisualLookupEditor.module.css'

// Incomplete FEA remains a persisted draft; retain its typed operands across
// mode/entry navigation for the lifetime of the owning document session.
const draftRules = new WeakMap<
  FeatureDraftSession,
  Map<string, { code: string; rule: VisualLookup }>
>()
function rememberedRule(
  session: FeatureDraftSession,
  id: string,
  code: string,
) {
  const cached = draftRules.get(session)?.get(id)
  return cached?.code === code ? cached.rule : parseVisualLookup(code)
}
function rememberRule(
  session: FeatureDraftSession,
  id: string,
  code: string,
  rule: VisualLookup,
) {
  const entries = draftRules.get(session) ?? new Map()
  entries.set(id, { code, rule })
  draftRules.set(session, entries)
}

const suggestedTag: Record<VisualLookupKind, string> = {
  'gsub-single': 'salt',
  'gsub-multiple': 'ccmp',
  'gsub-alternate': 'salt',
  'gsub-ligature': 'liga',
  'gsub-contextual': 'calt',
  'gpos-single': 'dist',
  'gpos-pair': 'kern',
  'gpos-mark-base': 'mark',
  'gpos-mark-mark': 'mkmk',
}

/** Source-backed forms share the same draft persistence and checked apply path. */
export function VisualLookupEditor({
  source,
  session,
  readOnly,
  selected,
  onSelect,
  master,
  generation,
  onSource,
}: {
  source: Features
  session: FeatureDraftSession
  readOnly: boolean
  selected: string | null
  onSelect: (id: string) => void
  master: string | null
  generation: number
  onSource: (id: string) => void
}) {
  const { t } = useTranslation()
  const [kind, setKind] = useState<VisualLookupKind>('gsub-single')
  const block =
    source.features.find((entry) => entry.id === selected) ?? source.features[0]
  const [form, setForm] = useState<{
    id: string | undefined
    code: string
    rule: VisualLookup | null
  }>(() => ({
    id: block?.id,
    code: block?.code ?? '',
    rule: block ? rememberedRule(session, block.id, block.code) : null,
  }))
  if (form.id !== block?.id || form.code !== (block?.code ?? '')) {
    setForm({
      id: block?.id,
      code: block?.code ?? '',
      rule: block ? rememberedRule(session, block.id, block.code) : null,
    })
  }
  const generated =
    !!block &&
    !!(
      block.automatic ||
      session
        .getSnapshot()
        .settings?.generated.some((entry) => entry.source_id === block.id)
    )
  const disabled = readOnly || generated
  const change = (next: Features) => {
    const settings = session.getSnapshot().settings
    if (settings) session.change(next, settings)
  }
  const update = (rule: VisualLookup) => {
    if (!block || disabled) return
    const code = serializeVisualLookup(rule)
    // Keep incomplete operands editable while their exact FEA is persisted.
    rememberRule(session, block.id, code, rule)
    setForm({ id: block.id, code, rule })
    change(updateSource(source, block.id, { code }))
  }
  const add = () => {
    if (readOnly) return
    const next = addSource(source, 'features', suggestedTag[kind])
    const added = next.features.at(-1)!
    const rule = defaultVisualLookup(kind)
    if (rule.kind === 'gpos-mark-base' || rule.kind === 'gpos-mark-mark')
      rule.markClass = `visual_${added.id}`
    const code = serializeVisualLookup(rule)
    rememberRule(session, added.id, code, rule)
    onSelect(added.id)
    setForm({ id: added.id, code, rule })
    change(updateSource(next, added.id, { code }))
  }
  const invalid = form.rule ? validateVisualLookup(form.rule).length > 0 : false
  return (
    <section className={styles.root} aria-label={t('visualLookup.title')}>
      <h3>{t('visualLookup.title')}</h3>
      <p className={styles.muted}>{t('visualLookup.help')}</p>
      <div className={styles.toolbar}>
        <label className={styles.field}>
          {t('visualLookup.newType')}
          <select
            value={kind}
            disabled={readOnly}
            onChange={(event) =>
              setKind(event.target.value as VisualLookupKind)
            }
          >
            {VISUAL_LOOKUP_KINDS.map((entry) => (
              <option key={entry} value={entry}>
                {t(`visualLookup.kind.${entry}`)}
              </option>
            ))}
          </select>
        </label>
        <button type="button" disabled={readOnly} onClick={add}>
          {t('visualLookup.add')}
        </button>
      </div>
      {block ? (
        <>
          <div className={styles.toolbar}>
            <label className={styles.field}>
              {t('visualLookup.feature')}
              <select
                value={block.id}
                onChange={(event) => onSelect(event.target.value)}
              >
                {source.features.map((entry, index) => (
                  <option key={entry.id} value={entry.id}>
                    {t('visualLookup.featureOption', {
                      tag: entry.tag,
                      number: index + 1,
                    })}
                  </option>
                ))}
              </select>
            </label>
            <label className={styles.field}>
              {t('features.featureTag')}
              <input
                value={block.tag}
                disabled={disabled}
                maxLength={4}
                onChange={(event) =>
                  change(
                    updateSource(source, block.id, { tag: event.target.value }),
                  )
                }
              />
            </label>
            <label>
              <input
                type="checkbox"
                checked={!block.disabled}
                disabled={disabled}
                onChange={(event) =>
                  change(
                    updateSource(source, block.id, {
                      disabled: !event.target.checked,
                    }),
                  )
                }
              />
              {t('features.enableSource')}
            </label>
            <button
              type="button"
              disabled={disabled}
              onClick={() => change(removeSource(source, block.id))}
            >
              {t('visualLookup.remove')}
            </button>
            <button type="button" onClick={() => onSource(block.id)}>
              {t('visualLookup.editSource')}
            </button>
          </div>
          {generated && (
            <p className={styles.muted}>{t('visualLookup.generated')}</p>
          )}
          {!isValidFeatureTag(block.tag) && (
            <p role="alert" className={styles.error}>
              {t('visualLookup.invalidTag')}
            </p>
          )}
          {form.rule ? (
            <>
              <h3>{t(`visualLookup.kind.${form.rule.kind}`)}</h3>
              <VisualLookupFields
                rule={form.rule}
                disabled={disabled}
                onChange={update}
              />
              {invalid && (
                <p role="alert" className={styles.error}>
                  {t('visualLookup.invalidOperands')}
                </p>
              )}
            </>
          ) : (
            <p>{t('visualLookup.unsupported')}</p>
          )}
          <VisualLookupPreview
            doc={session.doc}
            master={master}
            generation={generation}
            tag={block.tag}
            hasDraft={
              session.hasPendingEdits() ||
              !!session.getSnapshot().state?.has_unapplied_changes
            }
          />
          <h3>{t('visualLookup.sourcePreview')}</h3>
          <pre
            className={styles.preview}
            aria-label={t('visualLookup.sourcePreview')}
          >
            {block.code}
          </pre>
        </>
      ) : (
        <p>{t('visualLookup.empty')}</p>
      )}
    </section>
  )
}
