import { useTranslation } from 'react-i18next'
import type { Master } from '../../api'
import { Button } from '../../components/Button'
import styles from './hints.module.css'

export function CffPrivateSelector({
  master,
  selected,
  assignment,
  onSelect,
  onChange,
  onAssign,
}: {
  master: Master
  selected: number
  assignment: number | null
  onSelect: (index: number) => void
  onChange: (master: Master) => void
  onAssign: (index: number) => void
}) {
  const { t } = useTranslation()
  const dictionaries = [
    master.private_dict,
    ...(master.cff_private_dicts ?? []),
  ]
  const options = dictionaries.map((_, index) => (
    <option key={index} value={index}>
      {t('hints.privateDictionaryNumber', { index })}
    </option>
  ))
  return (
    <>
      <div className={styles.row}>
        <label>
          {t('hints.editPrivateDictionary')}
          <select
            value={selected}
            onChange={(event) => onSelect(Number(event.target.value))}
          >
            {options}
          </select>
        </label>
        <Button
          disabled={dictionaries.length >= 256}
          onClick={() => {
            onChange({
              ...master,
              cff_private_dicts: [
                ...(master.cff_private_dicts ?? []),
                { ...master.private_dict },
              ],
            })
            onSelect(dictionaries.length)
          }}
        >
          {t('hints.addPrivateDictionary')}
        </Button>
        {assignment !== null && (
          <label>
            {t('hints.glyphPrivateDictionary')}
            <select
              value={assignment}
              onChange={(event) => onAssign(Number(event.target.value))}
            >
              {options}
            </select>
          </label>
        )}
      </div>
      <p>{t('hints.cidPrivateHelp')}</p>
    </>
  )
}
