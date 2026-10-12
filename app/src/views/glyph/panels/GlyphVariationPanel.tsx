import { useCallback, useEffect, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  call,
  onDocumentEvent,
  type BracketCondition,
  type DesignSpaceCommand,
  type DocId,
  type Layer,
  type VariationPreview as Preview,
  type VariationState,
} from '../../../api'
import { Button } from '../../../components/Button'
import { useEditorStore } from '../../../stores/editorStore'
import { useVariationPreviewStore } from '../../../stores/variationPreviewStore'
import { errorNotice } from '../../../i18n/errors'
import { VariationPreview } from '../../fontinfo/VariationPreview'
import { userToDesign } from '../../fontinfo/designCoordinates'
import type { DesignSpaceValue } from '../../fontinfo/designSpaceSession'
import styles from '../../fontinfo/designSpace.module.css'
import { BracketConditionFields } from './BracketConditionFields'
import {
  axisTag,
  conditionSetError,
  conditionsOf,
  formatConditionSet,
} from './bracketConditions'
import { PanelSection } from './PanelSection'

/** A stable empty layer list, so the memo below has a stable dependency. */
const EMPTY_LAYERS: readonly Layer[] = []

/**
 * One alternate *region* of the glyph.
 *
 * A region is identified by its canonical condition set; every source that
 * shares the set belongs to it, and editing the region moves all of them
 * together so the sources never disagree about when they apply.
 */
interface Alternate {
  key: string
  conditions: BracketCondition[]
  sources: string[]
}

/**
 * Live interpolation and the glyph's conditional sources.
 *
 * The axis sliders scrub an interpolated overlay on the canvas without touching
 * the document. The condition editor below writes one `Layer` command per
 * gesture, so an alternate layer's condition set is one undo entry.
 */
export function GlyphVariationPanel({
  doc,
  glyph,
}: {
  doc: DocId
  glyph: string
}) {
  const { t } = useTranslation()
  const [variation, setVariation] = useState<VariationState | null>(null)
  const [error, setError] = useState<unknown>(null)
  const [refresh, setRefresh] = useState(0)
  const active = useEditorStore((store) => store.activeGlyph)
  const mine = active?.glyph.name === glyph && active.doc === doc
  const currentMaster = mine ? active.master : null
  const editableLayer = mine ? (active.outline?.layer ?? null) : null
  const reload = mine ? active.reload : undefined
  const axes = variation?.axes ?? []
  const state = useMemo<DesignSpaceValue | null>(
    () =>
      variation
        ? {
            ...variation,
            first_glyph: glyph,
            axes: variation.axes.map((axis) => ({
              ...axis,
              tag: axisTag(axis.tag),
            })),
          }
        : null,
    [variation, glyph],
  )
  const layers = mine ? active.glyph.layers : EMPTY_LAYERS
  const alternates = useMemo<Alternate[]>(() => {
    const regions = new Map<string, Alternate>()
    for (const layer of layers) {
      if (!('Bracket' in layer.kind)) continue
      const conditions = conditionsOf(layer.kind)
      const key = formatConditionSet(conditions)
      const region = regions.get(key)
      if (region) region.sources.push(layer.kind.Bracket.master)
      else
        regions.set(key, {
          key,
          conditions,
          sources: [layer.kind.Bracket.master],
        })
    }
    return [...regions.values()]
  }, [layers])
  const [editing, setEditing] = useState<string | null>(null)
  const [draft, setDraft] = useState<BracketCondition[]>([])
  const [busy, setBusy] = useState(false)
  const [conditionError, setConditionError] = useState('')
  const publishPreview = useCallback(
    (preview: Preview | null) => {
      const store = useVariationPreviewStore.getState()
      if (preview) store.show(doc, glyph, preview)
      else store.clear(doc, glyph)
    },
    [doc, glyph],
  )
  useEffect(
    () => () => useVariationPreviewStore.getState().clear(doc, glyph),
    [doc, glyph],
  )
  useEffect(() => {
    let request = 0
    let disposed = false
    const load = () => {
      const current = ++request
      void call('variation.state', { doc })
        .then((value) => {
          if (disposed || current !== request) return
          setVariation(value)
          setError(null)
        })
        .catch((cause: unknown) => {
          if (!disposed && current === request) setError(cause)
        })
    }
    load()
    const off = onDocumentEvent((event) => {
      if (event.doc === doc) load()
    })
    return () => {
      disposed = true
      off()
    }
  }, [doc, refresh])

  const primary = draft[0]
  const apply = (label: string, command: DesignSpaceCommand) => {
    if (!variation) return
    setBusy(true)
    setConditionError('')
    void call('variation.apply', {
      doc,
      expected_generation: variation.generation,
      label,
      command,
    })
      .then(() => {
        setEditing(null)
        setRefresh((value) => value + 1)
        reload?.()
      })
      .catch((cause: unknown) => {
        const notice = errorNotice(cause, t)
        setConditionError(notice.details ?? notice.message)
      })
      .finally(() => setBusy(false))
  }

  const applyConditions = (alternate: Alternate) => {
    const problem = conditionSetError(axes, draft)
    if (problem) {
      setConditionError(t(problem))
      return
    }
    // One command for the whole region: every source that shares the old set
    // moves to the new one, so the undo entry matches the gesture.
    apply(t('variation.editCondition'), {
      Conditions: {
        glyph,
        from: alternate.conditions,
        to: draft,
        name: formatConditionSet(draft),
      },
    })
  }

  const addConditionalSource = () => {
    const axis = axes[0]
    if (!editableLayer || !currentMaster || !axis) return
    const seeded: BracketCondition[] = [
      { axis: axis.tag, min: userToDesign(axis, axis.default) },
    ]
    apply(t('variation.addConditionalSource'), {
      Layer: {
        glyph,
        source: editableLayer,
        duplicate: true,
        kind: {
          Bracket: {
            master: currentMaster,
            axis: axis.tag,
            min: seeded[0]?.min ?? null,
            max: null,
            extra: [],
          },
        },
        name: formatConditionSet(seeded),
      },
    })
  }

  return (
    <PanelSection id="variation" title={t('variation.preview')}>
      {error != null && <p role="alert">{errorNotice(error, t).message}</p>}
      {state && (
        <div className={styles.editor}>
          <VariationPreview
            key={`${doc}:${glyph}`}
            doc={doc}
            state={state}
            fixedGlyph={glyph}
            onPreviewChange={publishPreview}
            onRefresh={() => setRefresh((value) => value + 1)}
          />
        </div>
      )}
      {axes.length > 0 && (
        <section aria-label={t('variation.conditionalSources')}>
          <h4>{t('variation.conditionalSources')}</h4>
          <p>{t('variation.conditionalSourcesHint')}</p>
          {alternates.length === 0 && (
            <p>{t('variation.noConditionalSources')}</p>
          )}
          <ul>
            {alternates.map((alternate) => (
              <li key={alternate.key}>
                <span>{alternate.key}</span>{' '}
                <small>
                  {t('variation.conditionSources', {
                    count: alternate.sources.length,
                  })}
                </small>{' '}
                <Button
                  disabled={busy}
                  onClick={() => {
                    const open = editing === alternate.key
                    setEditing(open ? null : alternate.key)
                    setDraft(open ? [] : alternate.conditions)
                    setConditionError('')
                  }}
                >
                  {t(
                    editing === alternate.key
                      ? 'common.close'
                      : 'variation.editCondition',
                  )}
                </Button>
                {editing === alternate.key && primary && (
                  <div className={styles.form}>
                    <BracketConditionFields
                      axes={axes}
                      axis={axisTag(primary.axis)}
                      min={primary.min ?? null}
                      max={primary.max ?? null}
                      location={{}}
                      extra={draft.slice(1)}
                      onAxis={(tag) => {
                        const next = axes.find(
                          (value) => axisTag(value.tag) === tag,
                        )
                        if (!next) return
                        setDraft(([first, ...rest]) =>
                          first ? [{ ...first, axis: next.tag }, ...rest] : [],
                        )
                      }}
                      onMin={(value) =>
                        setDraft(([first, ...rest]) =>
                          first
                            ? [{ ...first, min: value ?? undefined }, ...rest]
                            : [],
                        )
                      }
                      onMax={(value) =>
                        setDraft(([first, ...rest]) =>
                          first
                            ? [{ ...first, max: value ?? undefined }, ...rest]
                            : [],
                        )
                      }
                      onExtra={(value) =>
                        setDraft(([first]) => (first ? [first, ...value] : []))
                      }
                    />
                    <Button
                      variant="primary"
                      disabled={busy}
                      onClick={() => applyConditions(alternate)}
                    >
                      {t('variation.apply')}
                    </Button>
                  </div>
                )}
              </li>
            ))}
          </ul>
          <Button
            disabled={busy || !editableLayer || !currentMaster}
            onClick={addConditionalSource}
          >
            {t('variation.addConditionalSource')}
          </Button>
          {conditionError && <p role="alert">{conditionError}</p>}
        </section>
      )}
    </PanelSection>
  )
}
