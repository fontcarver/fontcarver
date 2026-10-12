import { useCallback, useLayoutEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { DocId, MasterId } from '../../api'
import { activeEditor } from '../../editor/activeEditor'
import { reportError } from '../../shell/commands'
import type { PositionedGlyph } from './textLayout'
import { identifyTextGaps } from './textGaps'
import { useTextKerning } from './useTextKerning'
import { parseTextInput } from './textInput'
import type { KerningMode } from './kerningModes'

interface Options {
  doc: DocId
  master: MasterId | null
  generation: number
  text: string
  direction: string | null
  features: Array<[string, boolean]>
  glyphs: readonly PositionedGlyph[]
  pending: boolean
  kerningMode?: KerningMode
  transition: (action: () => void) => Promise<void>
}

/** Connects exact shaped gap identities to the independently editable panel. */
export function useTextKerningControls(options: Options) {
  const { t } = useTranslation()
  const {
    doc,
    master,
    generation,
    text,
    direction,
    features,
    glyphs,
    pending,
    kerningMode = 'on',
    transition,
  } = options
  const identity = JSON.stringify([
    doc,
    master,
    text,
    direction,
    features,
    kerningMode,
  ])
  const current = useRef(identity)
  const selectionRevision = useRef(0)
  useLayoutEffect(() => {
    current.current = identity
  }, [identity])
  const [choice, setChoice] = useState<{
    identity: string
    index: number
    left: string
    right: string
  } | null>(null)
  const gaps = useMemo(
    () => identifyTextGaps(text, glyphs, direction),
    [text, glyphs, direction],
  )
  const prepare = useCallback(async () => {
    await activeEditor()?.prepareExternalEdit()
  }, [])
  const editor = useTextKerning({
    doc,
    master,
    generation,
    prepare,
    vertical: direction === 'ttb',
    punctuationShortcuts: kerningMode === 'locked',
  })
  const setPair = editor.panel.setPair
  const selectGap = useCallback(
    (index: number) => {
      const gap = gaps[index]
      if (
        kerningMode === 'off' ||
        pending ||
        !gap ||
        gap.reason !== null ||
        gap.left === null ||
        gap.right === null
      )
        return
      const selected = { identity, index, left: gap.left, right: gap.right }
      const revision = ++selectionRevision.current
      void transition(() => {
        if (
          current.current === identity &&
          revision === selectionRevision.current
        ) {
          setChoice(selected)
          void setPair({ left: selected.left, right: selected.right }).catch(
            reportError,
          )
        }
      }).catch(reportError)
    },
    [gaps, identity, kerningMode, pending, setPair, transition],
  )
  const selectedGap =
    choice?.identity === identity &&
    editor.panel.pair.left === choice.left &&
    editor.panel.pair.right === choice.right &&
    gaps[choice.index]?.left === choice.left &&
    gaps[choice.index]?.right === choice.right
      ? choice.index
      : null
  const disabledReason =
    kerningMode === 'off'
      ? t('kerningModes.offHint')
      : pending
        ? null
        : gaps.length > 0 && gaps.every((gap) => gap.reason !== null)
          ? t(`kerningCanvas.${gaps[0]!.reason}`)
          : glyphs.length === 1 &&
              parseTextInput(text).some(
                (token) =>
                  token.kind === 'literal' && Array.from(token.text).length > 1,
              )
            ? t('kerningCanvas.cluster')
            : null
  // Guard callbacks as well as disabled controls: a blur queued before a mode
  // transition must not commit a draft after kerning has been switched off.
  const enabled = useRef(kerningMode !== 'off')
  useLayoutEffect(() => {
    enabled.current = kerningMode !== 'off'
  }, [kerningMode])
  const panel = {
    ...editor.panel,
    editable: kerningMode !== 'off' && editor.panel.editable,
    status: editor.panel.status && {
      ...editor.panel.status,
      editable: kerningMode !== 'off' && editor.panel.status.editable,
    },
    setPair: async (pair: Parameters<typeof editor.panel.setPair>[0]) => {
      if (enabled.current) await editor.panel.setPair(pair)
    },
    setScope: async (scope: Parameters<typeof editor.panel.setScope>[0]) => {
      if (enabled.current) await editor.panel.setScope(scope)
    },
    setValue: (value: string) => {
      if (enabled.current) editor.panel.setValue(value)
    },
    commit: async () => {
      if (enabled.current) await editor.panel.commit()
    },
    remove: async () => {
      if (enabled.current) await editor.panel.remove()
    },
  }
  const handleKerningKey = editor.onKerningKey
  const onKerningKey = useCallback<typeof editor.onKerningKey>(
    (event) => {
      if (!enabled.current) return false
      if (
        (event.code === 'Comma' ||
          event.code === 'Period' ||
          event.key === ',' ||
          event.key === '.' ||
          event.key === '<' ||
          event.key === '>') &&
        selectedGap === null
      )
        return false
      return handleKerningKey(event)
    },
    [handleKerningKey, selectedGap],
  )
  return {
    ...editor,
    panel,
    onKerningKey,
    gaps,
    selectGap,
    selectedGap,
    disabledReason,
  }
}
