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
import {
  isDerivedProperty,
  knownPropertyType,
  KNOWN_PROPERTIES,
  parsePropertyValue,
  propertyProblem,
  propertyText,
  propertyTypeOf,
  renameProperty,
  type PropertyType,
} from './strikePropertiesModel'
import styles from './BitmapView.module.css'

/**
 * The strike's typed format properties.
 *
 * Standard XLFD and BDF names are offered with their declared type, custom
 * names can be added, renamed, retyped and removed, and every change is
 * validated here with the same rules the kernel enforces, so an unrepresentable
 * property is refused before it reaches the document rather than at export.
 */
export function StrikeProperties({
  doc,
  strike,
}: {
  doc: DocId
  strike: BitmapStrikeInfo
}) {
  const { t } = useTranslation(),
    [busy, setBusy] = useState(false)
  const [name, setName] = useState(''),
    [value, setValue] = useState(''),
    [type, setType] = useState<PropertyType>('string'),
    [error, setError] = useState(''),
    [renaming, setRenaming] = useState<string | null>(null),
    [renamed, setRenamed] = useState(''),
    [editing, setEditing] = useState<string | null>(null),
    [edited, setEdited] = useState('')
  const save = (properties: Record<string, BdfValue>, done?: () => void) => {
    if (busy) return
    setBusy(true)
    setError('')
    void withSettledDocument(doc, async () => {
      const current = await call('bitmap.state', { doc }),
        latest = current.strikes.find(
          (s) => s.ppem === strike.ppem && s.depth === strike.depth,
        )
      if (!latest) throw new Error(t('bitmap.strikeGone'))
      await call('bitmap.configure', {
        doc,
        expected_generation: current.generation,
        ...latest,
        properties,
      })
    })
      .then(() => done?.())
      .catch(reportError)
      .finally(() => setBusy(false))
  }
  const reject = (problem: string) => {
    setError(t(`bitmap.property.error.${problem}`))
  }
  const submit = () => {
    const typed = parsePropertyValue(type, value)
    if (!typed) {
      reject('notAnInteger')
      return
    }
    const problem = propertyProblem(name, typed)
    if (problem) {
      reject(problem)
      return
    }
    if (Object.hasOwn(strike.properties, name)) {
      reject('duplicate')
      return
    }
    save({ ...strike.properties, [name]: typed }, () => {
      setName('')
      setValue('')
    })
  }
  const commitEdit = (key: string) => {
    const typed = parsePropertyValue(
      propertyTypeOf(strike.properties[key]),
      edited,
    )
    if (!typed) {
      reject('notAnInteger')
      return
    }
    const problem = propertyProblem(key, typed)
    if (problem) {
      reject(problem)
      return
    }
    save({ ...strike.properties, [key]: typed }, () => setEditing(null))
  }
  const commitRename = (key: string) => {
    const next = renameProperty(strike.properties, key, renamed)
    if (typeof next === 'string') {
      reject(next)
      return
    }
    save(next, () => setRenaming(null))
  }
  const retype = (key: string) => {
    const current = strike.properties[key]
    const target = propertyTypeOf(current) === 'int' ? 'string' : 'int'
    const typed = parsePropertyValue(target, propertyText(current))
    if (!typed) {
      reject('notAnInteger')
      return
    }
    save({ ...strike.properties, [key]: typed })
  }
  return (
    <details>
      <summary>{t('bitmap.properties')}</summary>
      <StrikeMetrics
        key={`${strike.ppem}:${strike.depth}`}
        doc={doc}
        strike={strike}
      />
      <p className={styles.hint}>{t('bitmap.property.hint')}</p>
      {error && <p role="alert">{error}</p>}
      <dl>
        {Object.entries(strike.properties).map(([key, property]) => (
          <div key={key} className={styles.toolbar}>
            {renaming === key ? (
              <>
                <input
                  aria-label={t('bitmap.property.newName', { name: key })}
                  value={renamed}
                  onChange={(event) => setRenamed(event.target.value)}
                />
                <button disabled={busy} onClick={() => commitRename(key)}>
                  {t('bitmap.property.confirmRename')}
                </button>
                <button disabled={busy} onClick={() => setRenaming(null)}>
                  {t('common.cancel')}
                </button>
              </>
            ) : (
              <>
                <dt>
                  {key}
                  <small>
                    {' · '}
                    {t(`bitmap.property.type.${propertyTypeOf(property)}`)}
                    {isDerivedProperty(key) &&
                      ` · ${t('bitmap.property.derived')}`}
                  </small>
                </dt>
                {editing === key ? (
                  <>
                    <input
                      aria-label={t('bitmap.property.newValue', { name: key })}
                      value={edited}
                      onChange={(event) => setEdited(event.target.value)}
                    />
                    <button disabled={busy} onClick={() => commitEdit(key)}>
                      {t('bitmap.property.confirmValue')}
                    </button>
                    <button disabled={busy} onClick={() => setEditing(null)}>
                      {t('common.cancel')}
                    </button>
                  </>
                ) : (
                  <>
                    <dd>{propertyText(property)}</dd>
                    <button
                      disabled={busy}
                      aria-label={t('bitmap.property.edit', { name: key })}
                      onClick={() => {
                        setEditing(key)
                        setEdited(propertyText(property))
                        setError('')
                      }}
                    >
                      {t('bitmap.property.editShort')}
                    </button>
                    <button
                      disabled={busy}
                      aria-label={t('bitmap.property.rename', { name: key })}
                      onClick={() => {
                        setRenaming(key)
                        setRenamed(key)
                        setError('')
                      }}
                    >
                      {t('bitmap.property.renameShort')}
                    </button>
                    <button
                      disabled={busy || knownPropertyType(key) !== null}
                      aria-label={t('bitmap.property.retype', { name: key })}
                      onClick={() => retype(key)}
                    >
                      {t(
                        propertyTypeOf(property) === 'int'
                          ? 'bitmap.property.asString'
                          : 'bitmap.property.asInteger',
                      )}
                    </button>
                    <button
                      disabled={busy}
                      aria-label={t('bitmap.removeProperty', { name: key })}
                      onClick={() => {
                        const properties = { ...strike.properties }
                        delete properties[key]
                        save(properties)
                      }}
                    >
                      {t('bitmap.remove')}
                    </button>
                  </>
                )}
              </>
            )}
          </div>
        ))}
      </dl>
      <form
        className={styles.toolbar}
        onSubmit={(event) => {
          event.preventDefault()
          submit()
        }}
      >
        <label>
          {t('bitmap.propertyName')}
          <input
            required
            list="bitmap-known-properties"
            value={name}
            onChange={(event) => {
              const next = event.target.value
              setName(next)
              const known = knownPropertyType(next)
              if (known) setType(known)
            }}
          />
        </label>
        <datalist id="bitmap-known-properties">
          {KNOWN_PROPERTIES.map((entry) => (
            <option key={entry.name} value={entry.name} />
          ))}
        </datalist>
        <label>
          {t('bitmap.propertyValue')}
          <input
            required
            value={value}
            onChange={(event) => setValue(event.target.value)}
          />
        </label>
        <label>
          {t('bitmap.property.valueType')}
          <select
            value={type}
            disabled={knownPropertyType(name) !== null}
            onChange={(event) => setType(event.target.value as PropertyType)}
          >
            <option value="string">{t('bitmap.property.type.string')}</option>
            <option value="int">{t('bitmap.property.type.int')}</option>
          </select>
        </label>
        <button disabled={busy} type="submit">
          {t('bitmap.setProperty')}
        </button>
      </form>
    </details>
  )
}
