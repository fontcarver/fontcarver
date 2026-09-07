import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { COLOR_LABELS, colorLabelKey } from '../../font/colorLabels'
import { commitField } from '../commitField'

import styles from './panels.module.css'
import type { PanelContext } from './panelContext'
import {
  renameGlyphCommand,
  setGlyphMetaCommand,
  setUnicodesCommand,
} from './panelCommands'
import { TextRow } from './TextRow'

/** `U+0041`, the form the chips and the entry field both use. */
function formatScalar(value: number): string {
  return `U+${value.toString(16).toUpperCase().padStart(4, '0')}`
}

/**
 * A Unicode scalar typed as `41`, `0041`, `U+0041`, `u+41` or as the character
 * itself, or `null` when it is none of those. A single character is read as
 * itself so "type the letter" works, which is how every font editor's Unicode
 * field behaves.
 */
export function parseScalar(input: string): number | null {
  const text = input.trim()
  if (text === '') return null
  const hex = text.replace(/^u\+/i, '')
  if (/^[0-9a-f]{1,6}$/i.test(hex)) {
    const value = Number.parseInt(hex, 16)
    return value <= 0x10ffff ? value : null
  }
  const points = [...text]
  if (points.length === 1) return points[0]?.codePointAt(0) ?? null
  return null
}

/** Tags as the field shows them, and back again. */
export function parseTags(input: string): string[] {
  return [
    ...new Set(
      input
        .split(',')
        .map((tag) => tag.trim())
        .filter((tag) => tag !== ''),
    ),
  ]
}

/**
 * The Glyph section (design spec §7.1): identity, the derived classification
 * `fc-glyphdata` supplies, and the editable metadata `SetGlyphMeta` carries.
 */
export function GlyphPanel({ target, glyph, cell, reload }: PanelContext) {
  const { t } = useTranslation()
  const [draft, setDraft] = useState('')

  const meta = (
    update: Parameters<typeof setGlyphMetaCommand>[1],
    label: string,
  ) =>
    commitField(target, setGlyphMetaCommand(glyph.name, update), label, reload)

  const writeUnicodes = (unicodes: readonly number[]) =>
    commitField(
      target,
      setUnicodesCommand(glyph.name, unicodes),
      t('editor.undo.setUnicodes'),
      reload,
    )

  const addScalar = (input: string) => {
    const value = parseScalar(input)
    if (value === null || glyph.unicodes.includes(value)) return
    setDraft('')
    writeUnicodes([...glyph.unicodes, value].sort((a, b) => a - b))
  }

  const category = glyph.category ?? cell?.category ?? '—'
  const subCategory = glyph.sub_category ?? cell?.sub_category ?? '—'
  const script = glyph.script ?? cell?.script ?? '—'

  return (
    <>
      <TextRow
        label={t('glyphInspector.name')}
        value={glyph.name}
        required
        onCommit={(name) =>
          commitField(
            target,
            renameGlyphCommand(glyph.name, name),
            t('editor.undo.renameGlyph'),
            reload,
          )
        }
      />

      <div className={styles.row}>
        <span className={styles.rowLabel}>{t('glyphInspector.unicodes')}</span>
        <span className={styles.chips}>
          {glyph.unicodes.map((value) => (
            <span key={value} className={styles.chip}>
              {formatScalar(value)}
              <button
                type="button"
                className={styles.chipRemove}
                aria-label={t('panel.glyph.removeUnicode', {
                  value: formatScalar(value),
                })}
                onClick={() =>
                  writeUnicodes(
                    glyph.unicodes.filter((entry) => entry !== value),
                  )
                }
              >
                ×
              </button>
            </span>
          ))}
          <input
            className={styles.chipInput}
            value={draft}
            aria-label={t('panel.glyph.addUnicode')}
            placeholder="U+0041"
            onChange={(event) => setDraft(event.currentTarget.value)}
            onBlur={(event) => addScalar(event.currentTarget.value)}
            onKeyDown={(event) => {
              if (event.key !== 'Enter') return
              event.preventDefault()
              addScalar(event.currentTarget.value)
            }}
          />
        </span>
      </div>

      <div className={styles.row}>
        <span className={styles.rowLabel}>{t('glyphInspector.category')}</span>
        <span className={styles.rowValue}>{category}</span>
      </div>
      <div className={styles.row}>
        <span className={styles.rowLabel}>
          {t('glyphInspector.subCategory')}
        </span>
        <span className={styles.rowValue}>{subCategory}</span>
      </div>
      <div className={styles.row}>
        <span className={styles.rowLabel}>{t('glyphInspector.script')}</span>
        <span className={styles.rowValue}>{script}</span>
      </div>

      <div className={styles.row}>
        <span className={styles.rowLabel}>{t('glyphInspector.export')}</span>
        <button
          type="button"
          className={styles.toggle}
          aria-pressed={glyph.export}
          onClick={() =>
            meta({ export: !glyph.export }, t('editor.undo.setGlyphExport'))
          }
        >
          {glyph.export ? t('common.yes') : t('common.no')}
        </button>
      </div>

      <div className={styles.row}>
        <span className={styles.rowLabel}>{t('glyphInspector.color')}</span>
        <span className={styles.swatches}>
          <button
            type="button"
            className={`${styles.swatch} ${styles.swatchNone}`}
            aria-pressed={glyph.color_label === null}
            aria-label={t('panel.glyph.noColor')}
            title={t('panel.glyph.noColor')}
            onClick={() =>
              meta({ colorLabel: null }, t('editor.undo.setColorLabel'))
            }
          >
            ×
          </button>
          {COLOR_LABELS.map((label) => (
            <button
              key={label.index}
              type="button"
              className={styles.swatch}
              style={{ background: label.swatch }}
              aria-pressed={glyph.color_label === label.index}
              aria-label={t(`fontView.color.${colorLabelKey(label.index)}`)}
              title={t(`fontView.color.${colorLabelKey(label.index)}`)}
              onClick={() =>
                meta(
                  { colorLabel: label.index },
                  t('editor.undo.setColorLabel'),
                )
              }
            />
          ))}
        </span>
      </div>

      <TextRow
        label={t('glyphInspector.tags')}
        value={glyph.tags.join(', ')}
        placeholder={t('panel.glyph.tagsPlaceholder')}
        onCommit={(value) =>
          meta({ tags: parseTags(value) }, t('editor.undo.setGlyphTags'))
        }
      />
      <TextRow
        label={t('glyphInspector.note')}
        value={glyph.note}
        multiline
        onCommit={(note) => meta({ note }, t('editor.undo.setGlyphNote'))}
      />
    </>
  )
}
