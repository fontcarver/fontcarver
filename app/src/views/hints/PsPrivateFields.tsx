import { useTranslation } from 'react-i18next'
import type { Master, PsPrivate } from '../../api'
import { Button } from '../../components/Button'
import { NumberField } from '../../components/NumberField'
import { NumberListField } from './NumberListField'
import styles from './hints.module.css'

const ARRAYS = [
  'blue_values',
  'other_blues',
  'family_blues',
  'family_other_blues',
  'stem_snap_h',
  'stem_snap_v',
] as const
const NUMBERS = [
  'std_hw',
  'std_vw',
  'blue_scale',
  'blue_shift',
  'blue_fuzz',
] as const

export function PsPrivateFields({
  master,
  index = 0,
  onChange,
}: {
  master: Master
  index?: number
  onChange: (master: Master) => void
}) {
  const { t } = useTranslation()
  const privateDictionary =
    index === 0
      ? master.private_dict
      : (master.cff_private_dicts?.[index - 1] ?? master.private_dict)
  const changePrivate = (values: Partial<PsPrivate>) =>
    onChange(
      index === 0
        ? { ...master, private_dict: { ...privateDictionary, ...values } }
        : {
            ...master,
            cff_private_dicts: (master.cff_private_dicts ?? []).map(
              (item, i) => (i === index - 1 ? { ...item, ...values } : item),
            ),
          },
    )
  return (
    <>
      {index === 0 && (
        <>
          <h3>{t('hints.zones')}</h3>
          {master.alignment_zones.map((zone, index) => (
            <div className={styles.row} key={index}>
              {(['position', 'size'] as const).map((key) => (
                <NumberField
                  key={key}
                  label={t(`hints.${key}`)}
                  value={zone[key]}
                  onCommit={(value) =>
                    onChange({
                      ...master,
                      alignment_zones: master.alignment_zones.map((item, i) =>
                        i === index ? { ...item, [key]: value } : item,
                      ),
                    })
                  }
                />
              ))}
              <Button
                onClick={() =>
                  onChange({
                    ...master,
                    alignment_zones: master.alignment_zones.filter(
                      (_, i) => i !== index,
                    ),
                  })
                }
              >
                {t('hints.remove')}
              </Button>
            </div>
          ))}
          <Button
            onClick={() =>
              onChange({
                ...master,
                alignment_zones: [
                  ...master.alignment_zones,
                  { position: 0, size: -10 },
                ],
              })
            }
            disabled={master.alignment_zones.length >= 12}
          >
            {t('hints.addZone')}
          </Button>
          <h3>{t('hints.preferredStems')}</h3>
          {master.stems.map((stem, index) => (
            <div className={styles.row} key={index}>
              <label>
                {t('hints.name')}
                <input
                  value={stem.name}
                  onChange={(event) =>
                    onChange({
                      ...master,
                      stems: master.stems.map((item, i) =>
                        i === index
                          ? { ...item, name: event.target.value }
                          : item,
                      ),
                    })
                  }
                />
              </label>
              <label>
                <input
                  type="checkbox"
                  checked={stem.horizontal}
                  onChange={(event) =>
                    onChange({
                      ...master,
                      stems: master.stems.map((item, i) =>
                        i === index
                          ? { ...item, horizontal: event.target.checked }
                          : item,
                      ),
                    })
                  }
                />
                {t('hints.horizontal')}
              </label>
              <NumberField
                label={t('hints.width')}
                value={stem.width}
                onCommit={(value) =>
                  onChange({
                    ...master,
                    stems: master.stems.map((item, i) =>
                      i === index ? { ...item, width: value } : item,
                    ),
                  })
                }
              />
              <Button
                onClick={() =>
                  onChange({
                    ...master,
                    stems: master.stems.filter((_, i) => i !== index),
                  })
                }
              >
                {t('hints.remove')}
              </Button>
            </div>
          ))}
          <Button
            onClick={() =>
              onChange({
                ...master,
                stems: [
                  ...master.stems,
                  { name: '', horizontal: false, width: 80 },
                ],
              })
            }
            disabled={master.stems.length >= 24}
          >
            {t('hints.addStem')}
          </Button>
        </>
      )}
      <h3>{t('hints.private')}</h3>
      <p>{t('hints.privateHelp')}</p>
      <div className={styles.fields}>
        {ARRAYS.map((key) => (
          <NumberListField
            key={key}
            label={t(`hints.${key}`)}
            value={privateDictionary[key]}
            onChange={(value) => changePrivate({ [key]: value })}
          />
        ))}
        {NUMBERS.map((key) => (
          <label key={key}>
            {t(`hints.${key}`)}
            <input
              type="number"
              step="any"
              min={0}
              value={privateDictionary[key] ?? ''}
              onChange={(event) =>
                changePrivate({
                  [key]:
                    event.target.value === ''
                      ? null
                      : event.target.valueAsNumber,
                })
              }
            />
          </label>
        ))}
        <label>
          {t('hints.languageGroup')}
          <select
            value={privateDictionary.language_group ?? ''}
            onChange={(event) =>
              changePrivate({
                language_group:
                  event.target.value === '' ? null : Number(event.target.value),
              })
            }
          >
            <option value="">{t('hints.defaultValue')}</option>
            <option value="0">0</option>
            <option value="1">1</option>
          </select>
        </label>
        <label>
          {t('hints.forceBold')}
          <select
            value={
              privateDictionary.force_bold === null
                ? ''
                : String(privateDictionary.force_bold)
            }
            onChange={(event) =>
              changePrivate({
                force_bold:
                  event.target.value === ''
                    ? null
                    : event.target.value === 'true',
              })
            }
          >
            <option value="">{t('hints.defaultValue')}</option>
            <option value="true">{t('hints.enabled')}</option>
            <option value="false">{t('hints.disabled')}</option>
          </select>
        </label>
      </div>
    </>
  )
}
