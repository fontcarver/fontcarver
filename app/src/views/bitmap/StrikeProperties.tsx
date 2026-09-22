import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  call,
  type BdfValue,
  type BitmapStrikeInfo,
  type DocId,
} from '../../api'
import { withSettledDocument } from '../../editor/documentPersistence'
import { reportError } from '../../stores/uiStore'
import { StrikeMetrics } from './StrikeMetrics'
import styles from './BitmapView.module.css'
/** Explicit strike metadata form; untouched pixels stay entirely in the kernel. */
export function StrikeProperties({
  doc,
  strike,
}: {
  doc: DocId
  strike: BitmapStrikeInfo
}) {
  const { t } = useTranslation(),
    [busy, setBusy] = useState(false)
  const [key, setKey] = useState(''),
    [value, setValue] = useState(''),
    [numeric, setNumeric] = useState(false)
  const save = (patch: Partial<BitmapStrikeInfo>) => {
    if (busy) return
    setBusy(true)
    void withSettledDocument(doc, async () => {
      const current = await call('bitmap.state', { doc }),
        latest = current.strikes.find(
          (s) => s.ppem === strike.ppem && s.depth === strike.depth,
        )
      if (!latest) throw new Error('Bitmap strike no longer exists')
      await call('bitmap.configure', {
        doc,
        expected_generation: current.generation,
        ...latest,
        ...patch,
      })
    })
      .catch(reportError)
      .finally(() => setBusy(false))
  }
  return (
    <details>
      <summary>{t('bitmap.properties')}</summary>
      <StrikeMetrics
        key={`${strike.ppem}:${strike.depth}`}
        doc={doc}
        strike={strike}
      />
      <dl>
        {Object.entries(strike.properties).map(([name, property]) => (
          <div key={name} className={styles.toolbar}>
            <dt>{name}</dt>
            <dd>{'Int' in property ? property.Int : property.Str}</dd>
            <button
              disabled={busy}
              aria-label={t('bitmap.removeProperty', { name })}
              onClick={() => {
                const properties = { ...strike.properties }
                delete properties[name]
                save({ properties })
              }}
            >
              {t('bitmap.remove')}
            </button>
          </div>
        ))}
      </dl>
      <form
        className={styles.toolbar}
        onSubmit={(e) => {
          e.preventDefault()
          const property: BdfValue = numeric
            ? { Int: Number(value) }
            : { Str: value }
          save({ properties: { ...strike.properties, [key]: property } })
        }}
      >
        <label>
          {t('bitmap.propertyName')}
          <input
            required
            pattern="[A-Za-z_][A-Za-z0-9_]*"
            value={key}
            onChange={(e) => setKey(e.target.value)}
          />
        </label>
        <label>
          {t('bitmap.propertyValue')}
          <input
            required
            type={numeric ? 'number' : 'text'}
            value={value}
            onChange={(e) => setValue(e.target.value)}
          />
        </label>
        <label>
          <input
            type="checkbox"
            checked={numeric}
            onChange={(e) => setNumeric(e.target.checked)}
          />
          {t('bitmap.integer')}
        </label>
        <button disabled={busy} type="submit">
          {t('bitmap.setProperty')}
        </button>
      </form>
    </details>
  )
}
