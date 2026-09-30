import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  KERNING_SCOPES,
  kerningScopeRecord,
  kerningScopeSides,
  kerningSideName,
  type KerningScope,
  type TextKerningModel,
} from './kerningModel'
import styles from './TextKerningPanel.module.css'

/** Stored horizontal kerning controls, including explicit glyph-name entry. */
export function TextKerningPanel({
  model,
  gapDisabledReason,
  pending = false,
}: {
  model: TextKerningModel
  gapDisabledReason?: string | null
  pending?: boolean
}) {
  const { t } = useTranslation()
  const pairKey = JSON.stringify(model.pair)
  const [nameDraft, setNameDraft] = useState({
    pairKey,
    submitted: null as string | null,
    names: model.pair,
  })
  const names =
    nameDraft.pairKey === pairKey || nameDraft.submitted === pairKey
      ? nameDraft.names
      : model.pair
  if (nameDraft.pairKey !== pairKey && nameDraft.submitted !== pairKey) {
    setNameDraft({ pairKey, submitted: null, names: model.pair })
  }
  const setNames = (names: typeof model.pair) =>
    setNameDraft({ pairKey, submitted: nameDraft.submitted, names })
  const skipNameBlur = useRef(false)
  const resolution = model.resolution
  const exact = resolution ? kerningScopeRecord(resolution, model.scope) : null
  const choosePair = () => {
    if (
      names.left.trim() !== model.pair.left ||
      names.right.trim() !== model.pair.right
    ) {
      setNameDraft({
        pairKey,
        submitted: JSON.stringify({
          left: names.left.trim(),
          right: names.right.trim(),
        }),
        names,
      })
      void model.setPair(names)
    }
  }
  const status = model.status?.reason
    ? t(`kerningText.${model.status.reason}`)
    : model.loadFailed
      ? t('kerningText.loadFailed')
      : model.loading
        ? t('kerningText.loading')
        : !resolution
          ? t('kerningText.empty')
          : null

  return (
    <section
      className={styles.panel}
      aria-label={t('kerningText.title')}
      data-testid="text-kerning-panel"
    >
      <div className={styles.controls}>
        <span
          className={styles.heading}
          title={t('kerningText.classShortcutHint')}
        >
          {t('kerningText.title')}
        </span>
        {(['left', 'right'] as const).map((side) => (
          <input
            key={side}
            className={styles.name}
            type="text"
            spellCheck={false}
            aria-label={t(
              side === 'left'
                ? 'kerningText.firstGlyph'
                : 'kerningText.secondGlyph',
            )}
            data-testid={
              side === 'left' ? 'text-kerning-first' : 'text-kerning-second'
            }
            placeholder={t('kerningText.glyphPlaceholder')}
            value={names[side]}
            onChange={(event) =>
              setNames({ ...names, [side]: event.currentTarget.value })
            }
            onBlur={() => {
              if (skipNameBlur.current) skipNameBlur.current = false
              else choosePair()
            }}
            onKeyDown={(event) => {
              if (event.key === 'Enter') {
                event.preventDefault()
                choosePair()
              } else if (event.key === 'Escape') {
                event.preventDefault()
                skipNameBlur.current = true
                setNames(model.pair)
                event.currentTarget.blur()
              }
            }}
          />
        ))}
        <select
          className={styles.scope}
          aria-label={t('kerningText.scope')}
          data-testid="text-kerning-scope"
          value={model.scope}
          disabled={!model.status?.editable || !resolution}
          onChange={(event) => {
            void model.setScope(event.currentTarget.value as KerningScope)
          }}
        >
          {KERNING_SCOPES.map((scope) => (
            <option
              key={scope}
              value={scope}
              disabled={!resolution || !kerningScopeSides(resolution, scope)}
            >
              {t(`kerningText.scopes.${scope}`)}
            </option>
          ))}
        </select>
        <label className={styles.valueField}>
          <input
            className={styles.value}
            type="text"
            inputMode="decimal"
            aria-label={t('kerningText.value')}
            data-testid="text-kerning-value"
            value={model.value}
            disabled={!model.editable}
            aria-invalid={model.invalid || undefined}
            title={t(
              model.invalid
                ? 'kerningText.invalidValue'
                : 'kerningText.valueHint',
            )}
            onChange={(event) => model.setValue(event.currentTarget.value)}
            onBlur={() => {
              void model.commit()
            }}
            onKeyDown={(event) => {
              if (event.key === 'Enter') {
                event.preventDefault()
                void model.commit()
              } else if (event.key === 'Escape') {
                event.preventDefault()
                model.cancel()
                event.currentTarget.blur()
              }
            }}
          />
          <span>{t('kerningText.units')}</span>
        </label>
        <button
          type="button"
          className={styles.remove}
          disabled={!model.editable || !exact}
          title={t('kerningText.removeHint')}
          data-testid="text-kerning-remove"
          onClick={() => {
            void model.remove()
          }}
        >
          {t('kerningText.remove')}
        </button>
      </div>
      <div className={styles.details} aria-live="polite">
        {status && <span>{status}</span>}
        {resolution?.value !== null && resolution && (
          <>
            <span data-testid="text-kerning-resolved">
              {t('kerningText.resolved', { value: resolution.value })}
            </span>
            <span>
              {resolution.rule
                ? t('kerningText.winningRule', {
                    first: kerningSideName(resolution.rule.left),
                    second: kerningSideName(resolution.rule.right),
                  })
                : t('kerningText.noRule')}
            </span>
            <span>
              {exact
                ? t('kerningText.exactRecord', { value: exact.value })
                : t('kerningText.inherited')}
            </span>
          </>
        )}
        {model.invalid && (
          <span role="alert">{t('kerningText.invalidValue')}</span>
        )}
        {model.impactLoading && <span>{t('kerningText.impactLoading')}</span>}
        {model.impactFailed && <span>{t('kerningText.impactFailed')}</span>}
        {model.impact.map((group) => (
          <span key={group.name}>
            {t('kerningText.groupImpact', {
              name: group.name,
              count: group.total,
              members:
                group.members.join(', ') +
                (group.total > group.members.length ? '…' : ''),
            })}
          </span>
        ))}
        {(model.editing || pending) && (
          <span>
            {t(model.editing ? 'kerningText.saving' : 'kerningText.pending')}
          </span>
        )}
        {gapDisabledReason && <span>{gapDisabledReason}</span>}
      </div>
    </section>
  )
}
