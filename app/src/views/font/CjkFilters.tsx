import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { CjkFilter, DocId } from '../../api'
import { Panel } from '../../components/Panel'
import { useDocumentStore } from '../../stores/documentStore'
import styles from './cjk.module.css'

const CHARSETS = [
  'gb2312',
  'gb18030level1',
  'big5',
  'jisx0208',
  'ksx1001',
  'iicore',
] as const
const LANGUAGES = [
  'mandarin',
  'cantonese',
  'japanese',
  'korean',
  'any',
] as const
const EMPTY: CjkFilter = {
  radical: null,
  strokes: null,
  reading: null,
  language: null,
  charsets: [],
}

/** Offline Unihan filters, using every recorded stroke/radical alternative. */
export function CjkFilters({
  doc,
  value,
}: {
  doc: DocId
  value?: CjkFilter | null
}) {
  const { t } = useTranslation()
  const filter = value ?? EMPTY
  const [language, setLanguage] = useState(filter.language ?? 'mandarin')
  const setFilter = useDocumentStore((state) => state.setFilter)
  const update = (patch: Partial<CjkFilter>) => {
    const next = { ...filter, language, ...patch }
    const active =
      next.radical !== null ||
      next.strokes !== null ||
      !!next.reading ||
      next.charsets.length > 0
    setFilter(doc, { cjk: active ? next : null })
  }
  return (
    <Panel title={t('cjk.filters')}>
      <div className={styles.filters}>
        <label>
          {t('cjk.radical')}
          <select
            value={filter.radical ?? ''}
            onChange={(e) =>
              update({
                radical: e.target.value ? Number(e.target.value) : null,
              })
            }
          >
            <option value="">{t('cjk.anyRadical')}</option>
            {Array.from({ length: 214 }, (_, index) => (
              <option key={index} value={index + 1}>
                {index + 1} {String.fromCodePoint(0x2f00 + index)}
              </option>
            ))}
          </select>
        </label>
        <div className={styles.range}>
          {(['min', 'max'] as const).map((bound, index) => (
            <label key={bound}>
              {t(`cjk.strokes${bound === 'min' ? 'Min' : 'Max'}`)}
              <input
                type="number"
                min={0}
                max={255}
                value={filter.strokes?.[index] ?? ''}
                onChange={(e) => {
                  const text = e.target.value
                  if (!text) {
                    update({ strokes: null })
                    return
                  }
                  const number = Number(text)
                  if (!Number.isInteger(number) || number < 0 || number > 255)
                    return
                  const pair: [number, number] = filter.strokes
                    ? [...filter.strokes]
                    : [0, 255]
                  pair[index] = number
                  if (pair[0] > pair[1]) pair[1 - index] = number
                  update({ strokes: pair })
                }}
              />
            </label>
          ))}
        </div>
        <label>
          {t('cjk.language')}
          <select
            value={filter.language ?? language}
            onChange={(e) => {
              setLanguage(e.target.value)
              update({ language: e.target.value })
            }}
          >
            {LANGUAGES.map((language) => (
              <option key={language} value={language}>
                {t(`cjk.languages.${language}`)}
              </option>
            ))}
          </select>
        </label>
        <label>
          {t('cjk.reading')}
          <input
            type="search"
            maxLength={64}
            value={filter.reading ?? ''}
            onChange={(e) => update({ reading: e.target.value || null })}
          />
        </label>
        <label>
          {t('cjk.charset')}
          <select
            value={filter.charsets[0] ?? ''}
            onChange={(e) =>
              update({ charsets: e.target.value ? [e.target.value] : [] })
            }
          >
            <option value="">{t('cjk.anyCharset')}</option>
            {CHARSETS.map((charset) => (
              <option key={charset} value={charset}>
                {t(`cjk.charsets.${charset}`)}
              </option>
            ))}
          </select>
        </label>
        <p>{t('cjk.sourceHint')}</p>
      </div>
    </Panel>
  )
}
