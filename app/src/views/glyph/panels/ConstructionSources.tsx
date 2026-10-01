import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { call, type ConstructionCreateKind, type DocId } from '../../../api'
import { Button } from '../../../components/Button'
import { reportError } from '../../../shell/commands'

import styles from './layerEffects.module.css'

/**
 * Glyph-name prefix each attachment kind is conventionally named with.
 *
 * The kernel is authoritative (`fc_model::CONSTRUCTION_PREFIXES`); this is the
 * same table so the editor can offer the conventional glyphs for a kind the user
 * has chosen but not yet created. The prefix is a convention, not a rule: any
 * glyph can be attached, and a glyph named otherwise is only pointed out.
 */
export const CONSTRUCTION_PREFIXES: Record<
  ConstructionCreateKind['kind'],
  string
> = {
  corner: '_corner.',
  cap: '_cap.',
  segment: '_segment.',
  brush: '_brush.',
}

/** How many conventional sources the list offers. */
const LIMIT = 24

/** The conventionally named source glyphs for one attachment kind. */
export function ConstructionSources({
  doc,
  kind,
  source,
  onPick,
}: {
  doc: DocId
  kind: ConstructionCreateKind['kind']
  source: string
  onPick: (name: string) => void
}) {
  const { t } = useTranslation()
  const prefix = CONSTRUCTION_PREFIXES[kind]
  const [names, setNames] = useState<{ prefix: string; names: string[] }>({
    prefix: '',
    names: [],
  })

  useEffect(() => {
    let cancelled = false
    void call('glyphs.search', { doc, query: prefix, limit: LIMIT })
      .then((cells) => {
        if (cancelled) return
        setNames({
          prefix,
          names: cells
            .map((cell) => cell.name)
            .filter((name) => name.startsWith(prefix)),
        })
      })
      .catch(reportError)
    return () => {
      cancelled = true
    }
  }, [doc, prefix])

  const offered = names.prefix === prefix ? names.names : []
  return (
    <>
      <p>{t('construction.conventionHelp', { prefix })}</p>
      {offered.length > 0 ? (
        <div
          className={styles.row}
          role="group"
          aria-label={t('construction.conventionalSources', { prefix })}
        >
          {offered.map((name) => (
            <Button
              key={name}
              aria-pressed={name === source}
              onClick={() => onPick(name)}
            >
              {name}
            </Button>
          ))}
        </div>
      ) : (
        <p>{t('construction.noConventionalSources', { prefix })}</p>
      )}
      {source !== '' && !source.startsWith(prefix) && (
        <p role="status">
          {t('construction.unconventionalSource', { source, prefix })}
        </p>
      )}
    </>
  )
}
