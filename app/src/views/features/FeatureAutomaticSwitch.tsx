import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'

import type {
  AutomaticEntryOption,
  FeatureGenerationSettings,
  GeneratedFeatureRecord,
} from '../../api'
import { Dialog } from '../../components/Dialog'

import { automaticEntrySwitchable } from './automaticEntries'
import styles from './FeaturesView.module.css'
import type { SourceItem } from './featureSources'

/**
 * The per-entry automatic switch.
 *
 * A generated entry switches to manual by freezing exactly that entry, which
 * keeps its current text. A manual entry switches to automatic only after
 * confirming that its text will be replaced by the recipe's output.
 */
export function FeatureAutomaticSwitch({
  item,
  option,
  settings,
  generated,
  generatedSaved,
  readOnly,
  onManual,
  onAutomatic,
}: {
  item: SourceItem
  option: AutomaticEntryOption | undefined
  settings: FeatureGenerationSettings
  generated: GeneratedFeatureRecord | undefined
  generatedSaved: boolean
  readOnly: boolean
  onManual: (option: AutomaticEntryOption) => void
  onAutomatic: (option: AutomaticEntryOption) => void
}) {
  const { t } = useTranslation()
  const reasonId = useId()
  const [confirming, setConfirming] = useState(false)
  const switchable = !!option && automaticEntrySwitchable(option, settings)
  // A generated entry is always switchable back, even while a newer candidate
  // is still being previewed; freezing then waits for the saved snapshot.
  const disabled =
    readOnly || !option || (generated ? !generatedSaved : !switchable)
  const reason = !option
    ? t('features.automaticEntryUnavailable')
    : !switchable
      ? t('features.automaticEntryMapping')
      : generated && !generatedSaved
        ? t('features.freezePreview')
        : t('features.automaticEntryHelp')
  return (
    <>
      <label title={reason}>
        <input
          type="checkbox"
          disabled={disabled}
          checked={!!generated}
          aria-describedby={reasonId}
          onChange={() => {
            if (!option) return
            if (generated) {
              onManual(option)
            } else if (item.code.trim() === '') {
              onAutomatic(option)
            } else {
              setConfirming(true)
            }
          }}
        />
        {t('features.automaticEntry')}
      </label>
      {/* A disabled checkbox takes no focus and shows no tooltip, so the
          reason is also plain text next to it. */}
      <span id={reasonId} className={styles.muted}>
        {reason}
      </span>
      {confirming && !!option && (
        <Dialog
          title={t('features.automaticEntryConfirmTitle')}
          size="sm"
          onCancel={() => setConfirming(false)}
          onConfirm={() => {
            setConfirming(false)
            onAutomatic(option)
          }}
          footer={
            <>
              <button type="button" onClick={() => setConfirming(false)}>
                {t('common.cancel')}
              </button>
              <button
                type="button"
                className={styles.primary}
                onClick={() => {
                  setConfirming(false)
                  onAutomatic(option)
                }}
              >
                {t('features.automaticEntryConfirm')}
              </button>
            </>
          }
        >
          <p>
            {t('features.automaticEntryConfirmBody', {
              name: item.label,
              generator: t(`features.generator.${option.generator}`),
            })}
          </p>
        </Dialog>
      )}
    </>
  )
}
