import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { call, type DocId, type HintCommand, type HintState } from '../../api'
import { Button } from '../../components/Button'
import { Dialog } from '../../components/Dialog'
import { activeEditor } from '../../editor/activeEditor'
import { errorNotice } from '../../i18n/errors'
import { CffPrivateSelector } from './CffPrivateSelector'
import { PsFlexFields } from './PsFlexFields'
import { PsHintPreview } from './PsHintPreview'
import { PsPrivateFields } from './PsPrivateFields'
import { PsStemFields } from './PsStemFields'
import styles from './hints.module.css'

export function PsHintEditor({
  doc,
  master,
  glyph = null,
  layer = null,
  onClose,
  onApplied,
}: {
  doc: DocId
  master: string
  glyph?: string | null
  layer?: string | null
  onClose: () => void
  onApplied: () => void
}) {
  const { t } = useTranslation()
  const [state, setState] = useState<HintState | null>(null)
  const [original, setOriginal] = useState<HintState | null>(null)
  const [privateIndex, setPrivateIndex] = useState(0)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const form = useRef<HTMLFormElement>(null)
  useEffect(() => {
    let current = true
    void Promise.resolve(activeEditor()?.settle())
      .then(() => call('hints.state', { doc, master, glyph, layer }))
      .then((result) => {
        if (current) {
          setState(result)
          setOriginal(result)
          setPrivateIndex(result.cff_private_dict_index ?? 0)
        }
      })
      .catch((reason: unknown) => {
        if (current) {
          const notice = errorNotice(reason, t)
          setError(notice.details ?? notice.message)
        }
      })
    return () => {
      current = false
    }
  }, [doc, master, glyph, layer, t])
  const report = (reason: unknown) => {
    const notice = errorNotice(reason, t)
    setError(notice.details ?? notice.message)
  }
  const analyze = () => {
    if (
      !state ||
      !state.can_autohint_ps ||
      state.cff_private_dict_index ||
      !glyph ||
      busy
    )
      return
    setBusy(true)
    setError('')
    void call('hints.analyze', { doc, master, glyphs: [glyph] })
      .then((result) => {
        if (result.generation !== state.generation) {
          setError(t('hints.stale'))
          return
        }
        // Analysis also proposes values for every additional CID dictionary the
        // selection uses, so apply each one it returns.
        const dictionaries = [...(state.master.cff_private_dicts ?? [])]
        for (const [index, values] of result.cid_private)
          if (index >= 1 && index <= dictionaries.length)
            dictionaries[index - 1] = values
        setState({
          ...state,
          hints: result.hints[0]?.[1] ?? state.hints,
          master: {
            ...state.master,
            private_dict: result.private,
            cff_private_dicts: dictionaries,
          },
        })
      })
      .catch(report)
      .finally(() => setBusy(false))
  }
  const apply = () => {
    if (!state || !original || busy || !form.current?.reportValidity()) return
    const commands: HintCommand[] = []
    if (
      ['private_dict', 'alignment_zones', 'stems'].some(
        (key) =>
          JSON.stringify(state.master[key as keyof typeof state.master]) !==
          JSON.stringify(original.master[key as keyof typeof original.master]),
      )
    )
      commands.push({
        Private: {
          master,
          private: state.master.private_dict,
          zones: state.master.alignment_zones,
          stems: state.master.stems,
        },
      })
    for (const [index, privateDictionary] of (
      state.master.cff_private_dicts ?? []
    ).entries()) {
      if (
        JSON.stringify(privateDictionary) !==
        JSON.stringify(original.master.cff_private_dicts?.[index])
      )
        commands.push({
          CffPrivate: { master, index: index + 1, private: privateDictionary },
        })
    }
    if (
      glyph &&
      state.layer &&
      (state.cff_private_dict_index ?? 0) !==
        (original.cff_private_dict_index ?? 0)
    )
      commands.push({
        CffPrivateAssignment: {
          master,
          glyph,
          layer: state.layer,
          index: state.cff_private_dict_index ?? 0,
        },
      })
    if (
      glyph &&
      state.layer &&
      state.hints &&
      JSON.stringify(state.hints) !== JSON.stringify(original.hints)
    )
      commands.push({ Ps: { glyph, layer: state.layer, hints: state.hints } })
    if (commands.length === 0) {
      onClose()
      return
    }
    setBusy(true)
    setError('')
    void call('hints.apply', {
      doc,
      expected_generation: state.generation,
      commands,
      label: t('hints.editLabel'),
    })
      .then(() => {
        onApplied()
        onClose()
      })
      .catch(report)
      .finally(() => setBusy(false))
  }
  return (
    <Dialog
      title={t('hints.psTitle')}
      size="lg"
      scrollBody
      onCancel={() => {
        if (!busy) onClose()
      }}
      footer={
        <>
          <Button disabled={busy} onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button variant="primary" disabled={busy || !state} onClick={apply}>
            {t('variation.apply')}
          </Button>
        </>
      }
    >
      {error && <p role="alert">{error}</p>}
      {!state ? (
        <p>{t('glyphView.loading')}</p>
      ) : (
        <form
          className={styles.editor}
          ref={form}
          onSubmit={(event) => {
            event.preventDefault()
            apply()
          }}
        >
          <fieldset disabled={busy}>
            <legend>
              {state.master.name}
              {glyph ? ` · ${glyph}` : ''}
            </legend>
            {state.is_cid && (
              <CffPrivateSelector
                master={state.master}
                selected={privateIndex}
                assignment={state.cff_private_dict_index}
                onSelect={setPrivateIndex}
                onChange={(master) => setState({ ...state, master })}
                onAssign={(cff_private_dict_index) =>
                  setState({ ...state, cff_private_dict_index })
                }
              />
            )}
            {state.hints && (
              <>
                <PsHintPreview
                  path={state.path}
                  hints={state.hints}
                  upem={state.units_per_em}
                  flexPaths={state.flex_paths}
                />
                <div className={styles.row}>
                  <Button
                    disabled={
                      !state.can_autohint_ps ||
                      (state.cff_private_dict_index ?? 0) > 0 ||
                      JSON.stringify(state.master) !==
                        JSON.stringify(original?.master)
                    }
                    onClick={analyze}
                  >
                    {t('hints.analyze')}
                  </Button>
                  <Button
                    onClick={() =>
                      setState({
                        ...state,
                        hints: {
                          h_stems: [],
                          v_stems: [],
                          d_stems: [],
                          hint_masks: [],
                          flex: false,
                          flex_hints: [],
                          dot_sections: [],
                        },
                      })
                    }
                  >
                    {t('hints.clear')}
                  </Button>
                </div>
                <p>
                  {t(
                    (state.cff_private_dict_index ?? 0) > 0
                      ? 'hints.cidAnalysisUnavailable'
                      : state.can_autohint_ps
                        ? 'hints.analysisHelp'
                        : 'hints.masterLayerRequired',
                  )}
                </p>
                <PsStemFields
                  hints={state.hints}
                  onChange={(hints) => setState({ ...state, hints })}
                />
                <PsFlexFields
                  hints={state.hints}
                  contourCount={state.contour_count}
                  flexPaths={state.flex_paths}
                  onChange={(hints) => setState({ ...state, hints })}
                />
              </>
            )}
            <PsPrivateFields
              master={state.master}
              index={privateIndex}
              onChange={(master) => setState({ ...state, master })}
            />
          </fieldset>
        </form>
      )}
    </Dialog>
  )
}
